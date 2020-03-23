use skulpin::CoordinateSystem;
use skulpin::LogicalSize;
use skulpin::skia_safe;
use skia_safe::{Rect, Point, Canvas};

use skulpin::app::AppBuilder;
use skulpin::app::AppUpdateArgs;
use skulpin::app::AppDrawArgs;
use skulpin::app::AppError;
use skulpin::app::AppHandler;
use skulpin::app::VirtualKeyCode;
use skulpin::app::InputState;
use std::ffi::CString;

use skulpin_test::{JsonBuffer, JsonNode, JsonVariant, JsonBufferMode, JsonInput};

fn main() {
	// Setup logging
	/*env_logger::Builder::from_default_env()
	.filter_level(log::LevelFilter::Debug)
	.init();*/

	let example_app = ExampleApp::new();

	// Set up the coordinate system to be fixed at 900x600, and use this as the default window size
	// This means the drawing code can be written as though the window is always 900x600. The
	// output will be automatically scaled so that it's always visible.
	let logical_size = LogicalSize::new(900, 600);
	let visible_range = skulpin::skia_safe::Rect {
		left: 0.0,
		right: logical_size.width as f32,
		top: 0.0,
		bottom: logical_size.height as f32,
	};
	let scale_to_fit = skulpin::skia_safe::matrix::ScaleToFit::Center;

	AppBuilder::new()
		.app_name(CString::new("Skulpin Example App").unwrap())
		.use_vulkan_debug_layer(true)
		.inner_size(logical_size)
		.coordinate_system(CoordinateSystem::VisibleRange(visible_range, scale_to_fit))
		.run(example_app);
}

struct ExampleApp {
	pos: f32,
	buffer: JsonBuffer,
}

impl ExampleApp {
	pub fn new() -> Self {
		let json = JsonBuffer {
			nodes: vec![JsonNode {
				variant: JsonVariant::Array(vec![1, 2, 3]),
				parent: 0,
				left: 0,
				right: 0,
			}, JsonNode {
				variant: JsonVariant::Null,
				parent: 0,
				left: 0,
				right: 2,
			}, JsonNode {
				variant: JsonVariant::Null,
				parent: 0,
				left: 1,
				right: 3,
			}, JsonNode {
				variant: JsonVariant::Object(vec![4, 5]),
				parent: 0,
				left: 2,
				right: 0,
			}, JsonNode {
				variant: JsonVariant::ObjectEntry("name".to_string(), 6),
				parent: 3,
				left: 3,
				right: 5,
			}, JsonNode {
				variant: JsonVariant::ObjectEntry("age".to_string(), 7),
				parent: 3,
				left: 4,
				right: 3,
			}, JsonNode {
				variant: JsonVariant::String("Charlie Stanton".to_string()),
				parent: 4,
				left: 4,
				right: 4,
			}, JsonNode {
				variant: JsonVariant::Number(20.),
				parent: 5,
				left: 5,
				right: 5,
			}],
			selections: vec![3],
			mode: JsonBufferMode::Normal,
		};

		ExampleApp {
			pos: 0.0,
			buffer: json,
		}
	}
}

impl AppHandler for ExampleApp {
	fn update(
		&mut self,
		update_args: AppUpdateArgs,
	) {
		let input_state = update_args.input_state;
		let app_control = update_args.app_control;

		self.buffer.update(input_state);

		//self.pos = ((update_args.time_state.update_count() as f32 / 30.0).sin() + 1.0) / 2.0;

		if input_state.is_key_down(VirtualKeyCode::Q) {
			app_control.enqueue_terminate_process();
		}
	}

	fn draw(
		&mut self,
		draw_args: AppDrawArgs,
	) {
		let canvas = draw_args.canvas;

		// Generally would want to clear data every time we draw
		canvas.clear(skia_safe::Color::from_argb(0, 0, 0, 255));

		// Make a color to draw with
		let mut paint = skia_safe::Paint::new(skia_safe::Color4f::new(1.0, 0.0, 0.0, 1.0), None);
		paint.set_anti_alias(true);
		paint.set_style(skia_safe::paint::Style::Stroke);
		paint.set_stroke_width(2.0);

		// Draw a line
		canvas.draw_line(
			skia_safe::Point::new(100.0, 500.0),
			skia_safe::Point::new(800.0, 500.0),
			&paint,
		);

		// Draw a circle
		canvas.draw_circle(
			skia_safe::Point::new(200.0 + (self.pos * 500.0), 420.0),
			50.0,
			&paint,
		);

		self.buffer.draw(canvas);
	}


	fn fatal_error(
		&mut self,
		error: &AppError,
	) {
		println!("{}", error);
	}
}

fn rect_include_point(rect: &mut Rect, point: Point) {
	if rect.left > point.x {
		rect.left = point.x;
	} else if rect.right < point.x {
		rect.right = point.x;
	}
	if rect.top > point.y {
		rect.top = point.y;
	} else if rect.bottom < point.y {
		rect.bottom = point.y;
	}
}

struct TextBufferRenderer<'a> {
	indent: f32,
	line_num: f32,
	line_so_far: String,
	canvas: &'a mut Canvas,
	line_height: f32,
	character_width: f32,
	text_paint: &'a skia_safe::Paint,
	font: &'a skia_safe::Font,
	select_paint: &'a skia_safe::Paint,
	selections: Vec<Option<Rect>>,
	active_selections: Vec<usize>,
}

impl<'a> TextBufferRenderer<'a> {
	fn new<'b>(line_height: f32, character_width: f32, num_selections: usize, text_paint: &'b skia_safe::Paint, font: &'b skia_safe::Font, select_paint: &'b skia_safe::Paint, canvas: &'b mut Canvas) -> TextBufferRenderer<'b> {
		TextBufferRenderer {
			indent: 0.,
			line_num: 0.,
			line_so_far: "".to_string(),
			canvas: canvas,
			line_height: line_height,
			character_width: character_width,
			text_paint: text_paint,
			font: font,
			select_paint: select_paint,
			selections: (0..num_selections).map(|_| Option::None).collect(),
			active_selections: Vec::new(),
		}
	}
	fn add_to_selections(&mut self, point: Point) {
		for index in &self.active_selections {
			if let Some(Some(ref mut rect)) = self.selections.get_mut(*index) {
				rect_include_point(rect, point);
			} else {
				self.selections[*index] = Option::Some(Rect::new(point.x, point.y, point.x, point.y));
			}
		}
	}
	fn add_to_line(&mut self, to_add: &str) {
		let indent = self.indent*self.character_width*2.;
		let left = indent + (self.line_so_far.len() as f32) * self.character_width;
		self.line_so_far.push_str(to_add);
		let right = indent + (self.line_so_far.len() as f32) * self.character_width;
		let top = self.line_num*self.line_height;
		let bottom = top + self.line_height;
		self.add_to_selections(Point::new(left, top));
		self.add_to_selections(Point::new(right, bottom));
	}
	fn start_selection(&mut self, index: usize) {
		self.active_selections.push(index);
	}
	fn end_selection(&mut self, select: usize) {
		let index = self.active_selections.iter().position(|&s| s==select).unwrap();
		self.active_selections.remove(index);
	}
	fn draw_selections(&mut self) {
		for maybe_rect in &self.selections {
			if let Option::Some(rect) = maybe_rect {
				self.canvas.draw_rect(
					rect,
					self.select_paint,
				);
			}
		}
	}
	fn newline(&mut self) {
		let pos = Point::new(self.indent*self.character_width*2., self.line_height*(self.line_num + 1.));
		self.canvas.draw_str(self.line_so_far.as_str(), pos, self.font, self.text_paint);
		self.line_so_far = "".to_string();
		self.line_num += 1.;
	}
	fn indent(&mut self) {
		self.indent += 1.;
	}
	fn unindent(&mut self) {
		self.indent -= 1.;
	}
}

fn input_from_state(input_state: &InputState) -> Option<JsonInput> {
	if input_state.is_key_just_down(VirtualKeyCode::A) {
		Some(JsonInput::Char('a'))
	} else if input_state.is_key_just_down(VirtualKeyCode::B) {
		Some(JsonInput::Char('b'))
	} else if input_state.is_key_just_down(VirtualKeyCode::C) {
		Some(JsonInput::Char('c'))
	} else if input_state.is_key_just_down(VirtualKeyCode::D) {
		Some(JsonInput::Char('d'))
	} else if input_state.is_key_just_down(VirtualKeyCode::E) {
		Some(JsonInput::Char('e'))
	} else if input_state.is_key_just_down(VirtualKeyCode::F) {
		Some(JsonInput::Char('f'))
	} else if input_state.is_key_just_down(VirtualKeyCode::G) {
		Some(JsonInput::Char('g'))
	} else if input_state.is_key_just_down(VirtualKeyCode::H) {
		Some(JsonInput::Char('h'))
	} else if input_state.is_key_just_down(VirtualKeyCode::I) {
		Some(JsonInput::Char('i'))
	} else if input_state.is_key_just_down(VirtualKeyCode::J) {
		Some(JsonInput::Char('j'))
	} else if input_state.is_key_just_down(VirtualKeyCode::K) {
		Some(JsonInput::Char('k'))
	} else if input_state.is_key_just_down(VirtualKeyCode::L) {
		Some(JsonInput::Char('l'))
	} else if input_state.is_key_just_down(VirtualKeyCode::M) {
		Some(JsonInput::Char('m'))
	} else if input_state.is_key_just_down(VirtualKeyCode::N) {
		Some(JsonInput::Char('n'))
	} else if input_state.is_key_just_down(VirtualKeyCode::O) {
		Some(JsonInput::Char('o'))
	} else if input_state.is_key_just_down(VirtualKeyCode::P) {
		Some(JsonInput::Char('p'))
	} else if input_state.is_key_just_down(VirtualKeyCode::Q) {
		Some(JsonInput::Char('q'))
	} else if input_state.is_key_just_down(VirtualKeyCode::R) {
		Some(JsonInput::Char('r'))
	} else if input_state.is_key_just_down(VirtualKeyCode::S) {
		Some(JsonInput::Char('s'))
	} else if input_state.is_key_just_down(VirtualKeyCode::T) {
		Some(JsonInput::Char('t'))
	} else if input_state.is_key_just_down(VirtualKeyCode::U) {
		Some(JsonInput::Char('u'))
	} else if input_state.is_key_just_down(VirtualKeyCode::V) {
		Some(JsonInput::Char('v'))
	} else if input_state.is_key_just_down(VirtualKeyCode::W) {
		Some(JsonInput::Char('w'))
	} else if input_state.is_key_just_down(VirtualKeyCode::X) {
		Some(JsonInput::Char('x'))
	} else if input_state.is_key_just_down(VirtualKeyCode::Y) {
		Some(JsonInput::Char('y'))
	} else if input_state.is_key_just_down(VirtualKeyCode::Z) {
		Some(JsonInput::Char('z'))
	} else if input_state.is_key_just_down(VirtualKeyCode::Back) {
		Some(JsonInput::Backspace)
	} else if input_state.is_key_just_down(VirtualKeyCode::Space) {
		Some(JsonInput::Char(' '))
	} else {
		None
	}
}

trait Buffer {
	fn draw(&self, draw_args: &mut Canvas);
	fn update(&mut self, input_state: &InputState);
}

impl Buffer for JsonBuffer {
	fn draw<'a>(&self, canvas: &'a mut Canvas) {
		let mut font = skia_safe::Font::default();
		font.set_size(18.0);

		let mut text_paint = skia_safe::Paint::new(skia_safe::Color4f::new(1.0, 1.0, 1.0, 1.0), None);
		text_paint.set_anti_alias(true);
		text_paint.set_style(skia_safe::paint::Style::Fill);

		let mut select_paint = skia_safe::Paint::new(skia_safe::Color4f::new(1.0, 0., 0., 1.), None);
		select_paint.set_anti_alias(true);
		select_paint.set_style(skia_safe::paint::Style::Stroke);
		select_paint.set_stroke_width(1.);

		let mut renderer = TextBufferRenderer::new(18., 9., self.selections.len(), &text_paint, &font, &select_paint, canvas);

		let mut stack: Vec<(usize, bool, bool)> = vec![(0, false, false)];

		while let Some((cur, visited, comma)) = stack.pop() {
			let node = &self.nodes[cur];
			let node_selection_index = self.selections.iter().position(|&n| n==cur);
			if !visited {
				if let Some(nsi) = node_selection_index {
					renderer.start_selection(nsi);
				}
			}
			match &node.variant {
				JsonVariant::Null => {
					if !visited {
						renderer.add_to_line("null");
						if comma {
							renderer.add_to_line(",");
						}
						renderer.newline();
						stack.push((cur, true, false));
					}
				},
				JsonVariant::Bool(b) => {
					if !visited {
						renderer.add_to_line(b.to_string().as_str());
						if comma {
							renderer.add_to_line(",");
						}renderer.newline();
						stack.push((cur, true, false));
					}
				},
				JsonVariant::Number(num) => {
					if !visited {
						renderer.add_to_line(num.to_string().as_str());
						if comma {
							renderer.add_to_line(",");
						}
						renderer.newline();
						stack.push((cur, true, false));
					}
				},
				JsonVariant::String(string) => {
					if !visited {
						renderer.add_to_line(format!("\"{}\"", string).as_str());
						if comma {
							renderer.add_to_line(",");
						}
						renderer.newline();
						stack.push((cur, true, false));
					}
				},
				JsonVariant::ObjectEntry(key, value) => {
					if !visited {
						renderer.add_to_line(format!("\"{}\": ", key).as_str());
						stack.push((cur, true, false));
						stack.push((*value, false, comma));
					}
				},
				JsonVariant::Array(children) => {
					if !visited {
						renderer.add_to_line("[");
						stack.push((cur, true, comma));
						renderer.newline();
						renderer.indent();
						let mut child_has_comma = false;
						for child in children.iter().rev() {
							stack.push((*child, false, child_has_comma));
							child_has_comma = true;
						}
					} else {
						renderer.unindent();
						renderer.add_to_line("]");
						if comma {
							renderer.add_to_line(",");
						}
						renderer.newline();
					}
				},
				JsonVariant::Object(children) => {
					if !visited {
						renderer.add_to_line("{");
						stack.push((cur, true, comma));
						renderer.newline();
						renderer.indent();
						let mut child_has_comma = false;
						for child in children.iter().rev() {
							stack.push((*child, false, child_has_comma));
							child_has_comma = true;
						}
					} else {
						renderer.unindent();
						renderer.add_to_line("}");
						if comma {
							renderer.add_to_line(",");
						}
						renderer.newline();
					}
				},
			}
			if visited {
				if let Some(nsi) = node_selection_index {
					renderer.end_selection(nsi);
				}
			}
		}

		renderer.draw_selections();
	}
	fn update(&mut self, input_state: &InputState) {
		match self.mode {
			JsonBufferMode::Normal => {
				if input_state.is_key_just_down(VirtualKeyCode::K) {
					self.select_up();
				} else if input_state.is_key_just_down(VirtualKeyCode::J) {
					self.select_down();
				} else if input_state.is_key_just_down(VirtualKeyCode::H) {
					self.select_parent();
				} else if input_state.is_key_just_down(VirtualKeyCode::L) {
					self.select_first_child();
				} else if input_state.is_key_just_down(VirtualKeyCode::O) {
					self.new_down_sibling();
				} else if input_state.is_key_just_down(VirtualKeyCode::I) {
					self.mode = JsonBufferMode::Insert;
				} else if input_state.is_key_just_down(VirtualKeyCode::A) {
					self.objectify();
				} else if input_state.is_key_just_down(VirtualKeyCode::R) {
					self.new_first_child();
				}
			},
			JsonBufferMode::Insert => {
				if input_state.is_key_just_down(VirtualKeyCode::Escape) {
					self.mode = JsonBufferMode::Normal;
				} else {
					if let Some(input) = input_from_state(input_state) {
						self.input(input);
					}
				}
			},
		}
	}
}
