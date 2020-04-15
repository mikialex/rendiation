use crate::element::fragment::ElementFragment;

pub struct Slider{
    value: f32,
}

pub struct SliderComponent{
    dom: ElementFragment,

}

impl SliderComponent{
    fn new(slider: &Slider) {
        // build dom structure
    }

    fn event(){
        
    }
}

// Flex::row(vec![
//     Button::new()
// ])

// h(Flex::row(), 
//     vec![

//     ]
// )

// h(Button{
//     text: "demo",
//     width: 100,
//     height: 50,
// }, )

// fn create_ui(state: State){
//     h(Flex::row(300, 300), 
//         vec![
//             h(Button{
//                 text: state.text
//                 width: 100,
//                 height: 50,
//             }, )
//         ]
//     )
// }

// fn c(state: State){
//     Button::new()
//     .text(state.name)
//     .width(state.width)
//     .height(50)
//     .on_click<State>(|s|{
//         s.width+=1;
//     })
// }

// fn create_button(state: Button){
//     h(Quad::new(width, height))
// }

// fn test(){
//     data1 = Data::new(3);
//     data2 = Data::new(4);
//     data3 = Data::from2(data1, data2)
//     .compute(|d1, d2|{d1 + d2})

// }