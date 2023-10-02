# 实现记录

通用文件模型由下列对象类型组成。

- 超级块（superblock）对象：存放系统中已安装文件系统的有关信息。对于基于磁盘的文件系统，这类对象通常对应于存放在磁盘上的文件系统控制块，也就是说，每个文件系统都有一个超级块对象。
- 索引节点（inode）对象：存放关于具体文件的一般信息。对于基于磁盘的文件系统，这类对象通常对应于存放在磁盘上的文件控制块（FCB），也就是说，每个文件都有一个索引节点对象。每个索引节点对象都有一个索引节点号，这个号唯一地标识某个文件系统中的指定文件。
- 目录项（dentry）对象：存放目录项与对应文件进行链接的信息。VFS 把每个目录看作一个由若干子目录和文件组成的常规文件。例如，在查找路径名/tmp/test 时，内核为根目录“/”创建一个目录项对象，为根目录下的 tmp 项创建一个第 2 级目录项对象，为/tmp目录下的 test 项创建一个第 3 级目录项对象。
-  文件（file）对象：存放打开文件与进程之间进行交互的有关信息。这类信息仅当进程访问文件期间存在于内存中。



## 复杂实现

### Rename

```c
int renameat(int olddirfd, const char *oldpath,
                    int newdirfd, const char *newpath);
int renameat2(int olddirfd, const char *oldpath,
                    int newdirfd, const char *newpath, unsigned int flags);
```

1. 如果新文件存在，则进行原子替换：删除原目录下的旧目录项，修改新目录下的目录项信息，一般只需要修改指向的inode标识即可
2. 如果新文件不存在，则新建目录项
3. 如果新文件和旧文件都指向同一个inode，那么什么也不做，旧目录下的旧目录项会被删除
4. 如果旧文件是一个目录，那么新文件必须是一个空目录或者不存在
5. 如果旧文件是一个符号链接，那么只需要更新其内容，如果新文件是一个符号链接，那么更新新文件的内容



### 是否需要定义`VfsFile`接口？

在linux中，vfs层定义了File相关的接口，但在Linux中，已经定义了一个File数据结构来管理各个文件系统最后产生的文件。在rust中，定义一个File数据结构是不合适的，首先内核不一定需要你这个数据结构，其次，无法像C语言那样定义非常灵活的指针以指向不同的文件。每个文件系统一定会有自己的file接口实现，因此定义类似linux的file接口是有必要的。由于不同文件系统会被挂载到目录树的随机位置，当不同文件系统定义了实现file接口的数据结构，我们无法在创建文件系统时获得对应的类型，==这要求各个文件系统具备一个创建自己定义的file数据结构的接口==。一般来说，我们会调用inode的lookup接口来查找一个文件，这个接口一般返回的是实现Dentry结构的数据结构，因此，这个接口应该需要==根据Dentry数据结构来构造file数据结构==，这个接口的位置放在哪里最合适呢？

- 放在文件系统接口中？
- 放在超级块接口中？
- 放在inode/dentry接口中？这种形式不合适，因为这导致两者的语义发生了变形，他们是file的一部分，不应该由他们来创建file。

文件系统结构组织了对应文件系统所有的超级块，如果系统中具有多个ext3系统，则对应文件系统中包含了多个对应的超级块，因此这里考虑放在文件系统的接口中更为合适，我们可以从dentry结构中获取超级块结构，而超级块接口中我们可以找到其所属的文件系统结构，这样一来，就可以达到一个自底向上的构建过程。





### The Inode Cache

The inode cache is used to avoid reading and writing inodes to and from storage every time we need to read or update them. The cache uses a hash table and inodes are indexed with a hash function which takes as parameters the superblock (of a particular filesystem instance) and the inode number associated with an inode.

inodes are cached until either the filesystem is unmounted, the inode deleted or the system enters a memory pressure state. When this happens the Linux memory management system will (among other things) free inodes from the inode cache based on how often they were accessed.

- Caches inodes into memory to avoid costly storage operations
- An inode is cached until low memory conditions are triggered
- inodes are indexed with a hash table
- The inode hash function takes the superblock and inode number as inputs



Inode在内存中也存在缓存，除了可以加速访问之外，还可以完成不同文件共享同一个Inode的功能。

- 不同的文件可以对应同一个目录项结构     [重复打开同一个文件]
- 不同的目录项结构可以对应同一个Inode结构  [硬链接]

按照上文的描述，在具体的文件系统查找Inode的过程中，需要以超级块和inode number作为参数查找内存中存在的inode。这是一种与内核紧耦合的方式。我们需要考虑如何脱离这种耦合。

考虑到每个超级块代表一个文件系统实例，我们将这个缓存放置到超级块对象中，由超级块来管理各个文件系统中inode的缓存





### The Dentry Cache

- State:
  - Used – *d_inode* is valid and the *dentry* object is in use
  - Unused – *d_inode* is valid but the dentry object is not in use
  - Negative – *d_inode* is not valid; the inode was not yet loaded or the file was erased
- Dentry cache
  - List of used dentries (dentry->d_state == used)
  - List of the most recent used dentries (sorted by access time)
  - Hash table to avoid searching the tree





### unlink

`unlink`接口位于`VfsInode`中，其功能是用于删除一个文件的硬链接计数。但是这个删除不会导致系统中所有对象都被删除。具体而言，unlink的主要执行流程如下:

1. unlink首先在父dentry中查找对应名称的dentry
   1. 如果查找到这个dentry，则进一步调用父inode的unlink接口，文件的inode的硬链接计数会被减少，这可能会导致磁盘上的inode被删除掉，或者硬链接计数减1
   2. 如果没有查找到这个dentry，那么使用



Linux中有两个用于内存文件系统的主要选项：ramfs和tmpfs。这两者之间有一些区别

1. **ramfs（随机存取内存文件系统）**：
   - ramfs是一种简单的内存文件系统，它将所有数据存储在RAM中。
   - 它最初是在内核中引入的，但不进行任何存储空间限制。
   - 因为没有存储限制，ramfs在RAM用尽时会导致系统崩溃。

2. **tmpfs（临时文件系统）**：
   - tmpfs也是一种内存文件系统，但与ramfs不同，它会自动根据系统的可用RAM和交换空间来限制可用空间。
   - tmpfs通常用于创建临时文件系统，例如用于存储/tmp目录中的临时文件。
   - 当RAM用尽时，tmpfs可以将数据写入交换空间，从而避免系统崩溃。

总之，ramfs是一个非常基本的内存文件系统，而tmpfs更加灵活和适用于许多实际用途，因为它可以自动管理内存和交换空间。在实际应用中，通常更推荐使用tmpfs。
