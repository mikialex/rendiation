use std::cell::{RefCell, RefMut};
use std::rc::Rc;

struct DataWrap<T>{
    active_value: T,
    dirty: bool,
    last_value: T,
    // change_listeners: Vec<Box<dyn FnMut(&mut Any)>>
}

struct Data<T>(Rc<RefCell<DataWrap<T>>>);

struct StateGraph{

}

impl StateGraph{
    pub fn new() -> Self {
        Self{}
    }

}

impl<T: 'static + Clone> Data<T>{
    pub fn new(value: T) -> Self{
        let cloned = value.clone();
        let data = DataWrap {
            active_value: value,
            dirty: false,
            last_value: cloned
        };
        Data(Rc::new(RefCell::new(data)))
    }

    // pub fn get(&self) -> RefMut<T>{
    //     self.0.borrow_mut()
    // }

    pub fn set(&mut self, new_value: T){
        let mut self_borrow = self.0.borrow_mut();
        self_borrow.active_value = new_value;
        self_borrow.dirty = true;
    }

}

// struct ComputeData2<T1, T2>{
    
// }

// impl<T1, T2> ComputeData2<T1, T2>{
//     pub fn subscribe(v1: Data<T1>, v2: Data<T2>){
        
//     }

//     pub fn compute(exp: impl FnOnce(&T1, &T2)){

//     }
// }

// #[test]
// fn test(){
//     let mut states = StateGraph::new();

//     data1 = Data::new(3);
//     data2 = Data::new(4);
//     data3 = ComputeData2::subscribe(data1, data2)
//     .compute(|d1, d2|{d1 + d2})
// }

// fn c(state: State){
//     Button::new()
//     .text(state.name)
//     .width(state.width)
//     .height_raw(50)
//     .on_click::<State>(|s|{
//         s.width.set(s.width.get() + 1);
//     })
// }