use rendiation_webgpu::PipelineBuilder;

pub fn full_screen_vertex_shader(builder: &mut PipelineBuilder) {
  builder
    .include_vertex_entry(
      "
        [[stage(vertex)]]
        fn vs_main_full_screen(
          [[builtin(vertex_index)]] vertex_index: u32;
        ) -> VertexOutput {{
          var out: VertexOutput;
  
          switch (i32(input.vertex_index)) {{
            case 0: {{
              pos = vec2<f32>(left, top);
              out.position = input.tex_left_top;
            }}
            case 1: {{
              pos = vec2<f32>(right, top);
              out.position = vec2<f32>(input.tex_right_bottom.x, input.tex_left_top.y);
            }}
            case 2: {{
              pos = vec2<f32>(left, bottom);
              out.position = vec2<f32>(input.tex_left_top.x, input.tex_right_bottom.y);
            }}
            case 3: {{
              pos = vec2<f32>(right, bottom);
              out.position = input.tex_right_bottom;
            }}
          }}
  
          return out;
        }}
    ",
    )
    .use_vertex_entry("vs_main_full_screen");
}
