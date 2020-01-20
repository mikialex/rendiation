pub trait Component{

}

pub struct ComponentInstance<C>{
    state: C,
    document: Div
}

impl<C> ComponentInstance<C>{
    pub fn event(&mut self){
        
    }
}

pub struct TestCounter{
    count: usize,
}

impl TestCounter{
    fn render(&mut self){
        Div::new()
        .size(10, 10)
        .listener(
            Event::Click, |counter|{
                counter.add()
            }
        )
    }

    fn add(&mut self){
        self.add+=1;
    }
    
}
