# Generational arena shrink 的实现改进

这里所谓的arena， 指的是一种动态容器：

- 线性，可以用index/handle随机访问
- 可以创建删除，新创建的item，其index可能复用之前的

这里所谓的generational，是指容器还同时维护per index自增的generation信息，同时handle也存储有generation信息，该信息用来维护相同index的物体其handle的identity，即同一位置（index）的的item，无法被同一位置不同generation的handle说访问，这避免了重分配导致的可能的索引失效问题。

generational arena是无比有用的东西，然而其shrink有一些细节问题需要考虑：

- 在不放弃generational check的安全性的情况下，generation信息，即便在shrink之后，被shrink的部分依然需要保存
- arena内的空元素的位置是不确定的，所以shrink很难有效果， 而且shrink需要从最后往前依次找第一个非空元素

合理的实现讨论：

shrink之后，geneartion信息需要保存，一般做法就是真的去保存这个generation。毕竟一般来说item的data size比generation要大，所以分离generation的存储，然后不shrink generation buffer即可。

然而其实可以不用，因为generation是自增的，所以可以缩容的时候，记录被shrink部分的generation的最大值和size。扩容的时候，再把最大值以size count写出即可。这种做法牺牲了扩容和缩容的性能，但是可以保证generation check依然work。

shrink很难有效果，以及shrink的线性查找，是因为“洞”的位置不确定。那么解决这个问题，就是让“洞”的位置相对来说好确定一些。 这里我并不推荐使用堆来维护洞的位置，因为这导致插入和删除的成本都很高。我认为一种可行的改进做法是：

将empty list，拆分成高index和低index两个。在创建item时，优先pop低index，没有再pop高index，这样可以保证新创建的item总是尽可能在前面，而不是尾部。在删除item时，根据一个阈值（比如index是前60%）来决定是否应该push到哪个empty list。在任何时候，如果高index empty list是满的，那么整个高index可以直接shrink，否则再退化成线性查找，或者干脆放弃shrink。这其实是一个弱化版的（只有一层）的堆的实现，但是非常简单和有效。
