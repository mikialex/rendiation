// use crate::*;

// #[derive(Clone)]
// pub enum GPUResourceChange<T> {
//   Reference(T),
//   Content,
// }

// #[pin_project(project = MaterialGPUChangeProj)]
// pub enum KeyedRenderComponentDelta<T> {
//   Texture(T, #[pin] Option<ReactiveGPU2DTextureView>),
//   // Uniform(T,),  we don't have shared this now
//   // Vertex(T,),  we don't have shared this now
//   OwnedBindingContent,
//   ShaderHash,
// }

// pub enum FlattenedKeyedRenderComponentDelta<T> {
//   TextureRef(T, Option<GPU2DTextureView>),
//   // Uniform(T,),  we don't have shared this now
//   // Vertex(T,),  we don't have shared this now
//   Content,
//   ShaderHash,
// }

pub enum RenderComponentDelta {
  ShaderHash,
  ContentRef,
  Content,
  Draw,
}

// impl<T: Copy> Stream for KeyedRenderComponentDelta<T> {
//   type Item = FlattenedKeyedRenderComponentDelta<T>;

//   fn poll_next(
//     self: __core::pin::Pin<&mut Self>,
//     cx: &mut task::Context<'_>,
//   ) -> task::Poll<Option<Self::Item>> {
//     Poll::Ready(Some(match self.project() {
//       MaterialGPUChangeProj::Texture(key, stream) => {
//         return if let Some(stream) = *stream.as_mut() {
//           if let Poll::Ready(r) = stream.poll_next_unpin(cx) {
//             if let Some(r) = r {
//               match r {
//                 GPUResourceChange::Content => {
//                   Poll::Ready(Some(FlattenedKeyedRenderComponentDelta::Content))
//                 }
//                 GPUResourceChange::Reference(tex) => Poll::Ready(Some(
//                   FlattenedKeyedRenderComponentDelta::TextureRef(*key, Some(tex)),
//                 )),
//               }
//             } else {
//               Poll::Ready(None)
//             }
//           } else {
//             Poll::Pending
//           }
//         } else {
//           Poll::Ready(Some(FlattenedKeyedRenderComponentDelta::TextureRef(
//             *key, None,
//           )))
//         }
//       }
//       MaterialGPUChangeProj::OwnedBindingContent => FlattenedKeyedRenderComponentDelta::Content,
//       MaterialGPUChangeProj::ShaderHash => FlattenedKeyedRenderComponentDelta::ShaderHash,
//     }))
//   }
// }

// use __core::{
//   pin::Pin,
//   task::{Context, Poll},
// };
// use pin_project::pin_project;
// #[pin_project]
// struct MaterialGPUReactiveCell<T: WebGPUMaterialIncremental> {
//   weak_source: SceneItemWeakRef<T>,
//   gpu: T::GPU,
//   #[pin]
//   stream: T::Stream,
// }
