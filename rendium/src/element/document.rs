
pub type ElementHandle = ArenaTreeNodeHandle<Element>;

pub struct Document{
    tree: ArenaTree<Element>,
    active_element: Option<ElementHandle>,
    hovering_element: Option<ElementHandle>,
    event: EventHub,
}

impl Document {
    pub fn get_display_list() -> DisplayList{

    }
}

pub struct Element{
    
}
