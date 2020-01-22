
use crate::index_container::IndexContainer;

pub struct Tree<T>{
    root: usize,
    children: Vec<Vec<usize>>,
    parent: Vec<usize>,
    items: IndexContainer<T>
}

struct IndexWrap<T>{
    item: T,
    index: usize
}

impl<T> IndexWrap<T>{
    pub fn get_index(&self) -> usize{
        self.index
    }
}

impl<T> Tree<T>{
    pub fn new() -> Self {
        Self {
            root: 0,
            children: vec![],
            parent: vec![],
            items: IndexContainer::new()
        }
    }

    // pub fn set_item(item: T){

    // }

    // pub fn delete_item(){

    // }

    // pub fn get_free_index(&mut self) -> usize{
    //     if let Some(id) = self.free_list.pop() {
    //         return id;
    //     }
    //     let id = self.children.len();
    //     self.children.push(vec![]);
    //     self.parent.push(id);
    //     id
    // }

    pub fn append_child(&mut self, parent: usize, child: usize) {
        self.children[parent].push(child);
        self.parent[child] = parent;
    }

    pub fn remove_child(&mut self, parent: usize, child: usize) {
        let ix = self.children[parent]
            .iter()
            .position(|&x| x == child)
            .expect("tried to remove nonexistent child");
        self.children[parent].remove(ix);
        self.parent[child] = child;
    }
}

// struct UITree{
//     hierachy: Tree,
//     items: Vec<(Box<dyn Element>, usize)>
// }

// impl UITree{
//     // pub fn new() -> Self{
//     //     Self{

//     //     }
//     // }
// }
