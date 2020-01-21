use crate::element::Div;

pub trait Component<C>{
    fn render(&self) -> Div<C>;
}

pub struct ComponentInstance<C: Component<C>>{
    state: C,
    document: Div<C>
}

impl<C: Component<C>> ComponentInstance<C>{
    pub fn new(state: C)-> Self{
        let document = state.render();
        ComponentInstance{
            state,
            document
        }
    }
    pub fn event(&mut self){
        
    }
}

//
//
// user code

pub struct TestCounter{
    count: usize,
    sub_item: bool,
}

impl TestCounter{
    fn add(&mut self){
        self.count+=1;
    }
    
}

impl Component<TestCounter> for TestCounter{
    fn render(&self) -> Div<Self>{
        let mut div = Div::new();
        div.listener(
            |_, counter: &mut Self|{
                counter.add()
            }
        );
        div
    }
}

//
// more big

// pub struct MyUI{
//     counter: TestCounter,
//     id: usize
// }


// impl Component<MyUI> for MyUI{
//     fn render(&self) -> Div<Self>{
//         let mut div = Div::new();
//         div.listener(
//             |_, counter: &mut Self|{
//                 counter.add()
//             }
//         );
//         div
//     }
// }