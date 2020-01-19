pub trait Component{

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
