use crate::*;

pub struct TraverseIter<T, F> {
  pub(crate) visit_stack: Vec<T>,
  pub(crate) visit_decider: F,
}

impl<T, F> Iterator for TraverseIter<T, F>
where
  T: AbstractTreeNode + Clone,
  F: FnMut(&T) -> NextTraverseVisit,
{
  type Item = T;

  fn next(&mut self) -> Option<Self::Item> {
    if let Some(to_visit) = self.visit_stack.pop() {
      match (self.visit_decider)(&to_visit) {
        NextTraverseVisit::Exit => None,
        NextTraverseVisit::VisitChildren => {
          to_visit.visit_children(|child| self.visit_stack.push(child.clone()));
          Some(to_visit)
        }
        NextTraverseVisit::SkipChildren => Some(to_visit),
      }
    } else {
      None
    }
  }
}

/// In fact I don't know if it's essential to keep mutable version
pub struct TraverseMutIter<T, F> {
  pub(crate) visit_stack: Vec<T>,
  pub(crate) visit_decider: F,
}

impl<T, F> Iterator for TraverseMutIter<T, F>
where
  T: AbstractTreeMutNode + Clone,
  F: FnMut(&T) -> NextTraverseVisit,
{
  type Item = T;

  fn next(&mut self) -> Option<Self::Item> {
    if let Some(mut to_visit) = self.visit_stack.pop() {
      match (self.visit_decider)(&to_visit) {
        NextTraverseVisit::Exit => None,
        NextTraverseVisit::VisitChildren => {
          to_visit.visit_children_mut(|child| self.visit_stack.push(child.clone()));
          Some(to_visit)
        }
        NextTraverseVisit::SkipChildren => Some(to_visit),
      }
    } else {
      None
    }
  }
}
