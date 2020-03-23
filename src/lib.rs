use std::iter::once;

use skulpin::app::AppDrawArgs;
use skulpin::skia_safe::{Point, Rect};

pub enum JsonBufferMode {
	Normal,
	Insert,
}

pub enum JsonVariant {
	Null,
	Bool(bool),
	Number(f64),
	String(String),
	ObjectEntry(String, usize),
	Array(Vec<usize>),
	Object(Vec<usize>),
}

pub enum JsonInput {
	Char(char),
	Backspace,
}

pub struct JsonNode {
	pub variant: JsonVariant,
	pub parent: usize,
	pub left: usize,
	pub right: usize,
}

pub struct JsonBuffer {
	pub nodes: Vec<JsonNode>,
	pub selections: Vec<usize>,
	pub mode: JsonBufferMode,
}

impl JsonBuffer {
	pub fn select_up(&mut self) {
		let new_selections = self.selections.iter().map(|index| {
			self.nodes[*index].left
		});
		self.selections = new_selections.collect();
	}
	pub fn select_down(&mut self) {
		let new_selections = self.selections.iter().map(|index| {
			self.nodes[*index].right
		});
		self.selections = new_selections.collect();
	}
	pub fn select_parent(&mut self) {
		let new_selections = self.selections.iter().map(|index| {
			self.nodes[*index].parent
		});
		self.selections = new_selections.collect();
	}
	pub fn select_first_child(&mut self) {
		let new_selections = self.selections.iter().map(|index| {
			match self.nodes[*index].variant {
				JsonVariant::ObjectEntry(_, child) => {
					child
				},
				JsonVariant::Array(ref children) => {
					if children.len() > 0 {
						children[0]
					} else {
						*index
					}
				},
				JsonVariant::Object(ref children) => {
					if children.len() > 0 {
						children[0]
					} else {
						*index
					}
				},
				_ => *index,
			}
		});
		self.selections = new_selections.collect();
	}
	pub fn select_all_children(&mut self) {
		let new_selections = self.selections.iter().flat_map(|index: &usize| {
			let iter: Box<dyn Iterator<Item=usize>> = match &self.nodes[*index].variant {
				JsonVariant::Null
					| JsonVariant::Bool(_)
					| JsonVariant::Number(_)
					| JsonVariant::String(_) => Box::new(once(*index)),
				JsonVariant::ObjectEntry(_, child) => Box::new(once(*child)),
				JsonVariant::Array(children) => Box::new(children.iter().map(|c: &usize| *c)),
				JsonVariant::Object(children) => Box::new(children.iter().map(|c: &usize| *c)),
			};
			iter
		});
		self.selections = new_selections.collect();
	}
	pub fn new_first_child(&mut self) {
		let mut new_selections = Vec::with_capacity(self.selections.len());
		for selection_index in self.selections.iter() {
			let new_index = self.nodes.len();
			match self.nodes[*selection_index].variant {
				JsonVariant::Array(ref mut children) => {
					if children.len() == 0 {
						children.push(new_index);
						self.nodes.push(JsonNode {
							variant: JsonVariant::Null,
							parent: *selection_index,
							left: *selection_index,
							right: *selection_index,
						});
					} else {
						let old_first = children[0];
						children.insert(0, new_index);
						self.nodes.push(JsonNode {
							variant: JsonVariant::Null,
							parent: *selection_index,
							left: *selection_index,
							right: old_first,
						});
						self.nodes[old_first].left = new_index;
					}
					new_selections.push(new_index);
				},
				JsonVariant::Object(ref mut children) => {
					if children.len() == 0 {
						children.push(new_index);
						self.nodes.push(JsonNode {
							variant: JsonVariant::ObjectEntry("".to_string(), new_index+1),
							parent: *selection_index,
							left: *selection_index,
							right: *selection_index,
						});
					} else {
						let old_first = children[0];
						children.insert(0, new_index);
						self.nodes.push(JsonNode {
							variant: JsonVariant::ObjectEntry("".to_string(), new_index+1),
							parent: *selection_index,
							left: *selection_index,
							right: old_first,
						});
						self.nodes[old_first].left = new_index;
					}
					self.nodes.push(JsonNode {
						variant: JsonVariant::Null,
						parent: new_index,
						left: new_index,
						right: new_index,
					});
					new_selections.push(new_index);
				},
				_ => {
					new_selections.push(*selection_index);
				},
			}
		};
		self.selections = new_selections;
	}
	pub fn new_up_sibling(&mut self) {
		let mut new_selections: Vec<usize> = Vec::with_capacity(self.selections.len());
		for selection_index in self.selections.iter() {
			let new_index = self.nodes.len();
			let parent_index = self.nodes[*selection_index].parent;
			if parent_index != *selection_index {
				let parent = &mut self.nodes[parent_index];
				match parent.variant {
					JsonVariant::Array(ref mut children) => {
						let target_index = children.iter().position(|&i| i==*selection_index).unwrap();
						children.insert(target_index, new_index);
						let right_index = children[target_index+1];
						let mut left_index = parent_index;
						if target_index != 0 {
							left_index = children[target_index-1];
							self.nodes[left_index].right = new_index;
						}
						self.nodes[right_index].left = new_index;
						self.nodes.push(JsonNode {
							variant: JsonVariant::Null,
							parent: parent_index,
							left: left_index,
							right: right_index,
						});
						new_selections.push(new_index);
					},
					JsonVariant::Object(ref mut children) => {
						let target_index = children.iter().position(|&i| i==*selection_index).unwrap();
						children.insert(target_index, new_index);
						let right_index = children[target_index+1];
						let mut left_index = parent_index;
						if target_index != 0 {
							left_index = children[target_index-1];
							self.nodes[left_index].right = new_index;
						}
						self.nodes[right_index].left = new_index;
						self.nodes.push(JsonNode {
							variant: JsonVariant::ObjectEntry("".to_string(), new_index+1),
							parent: parent_index,
							left: left_index,
							right: right_index,
						});
						self.nodes.push(JsonNode {
							variant: JsonVariant::Null,
							parent: new_index,
							left: new_index,
							right: new_index,
						});
						new_selections.push(new_index);
					},
					_ => {
						new_selections.push(*selection_index);
					},
				}
			} else {
				new_selections.push(*selection_index);
			}
		}
		self.selections = new_selections;
	}
	pub fn new_down_sibling(&mut self) {
		let mut new_selections: Vec<usize> = Vec::with_capacity(self.selections.len());
		for selection_index in self.selections.iter() {
			let new_index = self.nodes.len();
			let parent_index = self.nodes[*selection_index].parent;
			if parent_index != *selection_index {
				let parent = &mut self.nodes[parent_index];
				match parent.variant {
					JsonVariant::Array(ref mut children) => {
						let target_index = children.iter().position(|&i| i==*selection_index).unwrap()+1;
						children.insert(target_index, new_index);
						let mut right_index = parent_index;
						let left_index = children[target_index-1];
						if target_index+1 != children.len() {
							right_index = children[target_index+1];
							self.nodes[right_index].left = new_index;
						}
						self.nodes[left_index].right = new_index;
						self.nodes.push(JsonNode {
							variant: JsonVariant::Null,
							parent: parent_index,
							left: left_index,
							right: right_index,
						});
						new_selections.push(new_index);
					},
					JsonVariant::Object(ref mut children) => {
						let target_index = children.iter().position(|&i| i==*selection_index).unwrap()+1;
						children.insert(target_index, new_index);
						let mut right_index = parent_index;
						let left_index = children[target_index-1];
						if target_index+1 != children.len() {
							right_index = children[target_index+1];
							self.nodes[right_index].left = new_index;
						}
						self.nodes[left_index].right = new_index;
						self.nodes.push(JsonNode {
							variant: JsonVariant::ObjectEntry("".to_string(), new_index+1),
							parent: parent_index,
							left: left_index,
							right: right_index,
						});
						self.nodes.push(JsonNode {
							variant: JsonVariant::Null,
							parent: new_index,
							left: new_index,
							right: new_index,
						});
						new_selections.push(new_index);
					},
					_ => {
						new_selections.push(*selection_index);
					},
				}
			} else {
				new_selections.push(*selection_index);
			}
		}
		self.selections = new_selections;
	}
	pub fn input(&mut self, input: JsonInput) {
		for selection_index in self.selections.iter() {
			match self.nodes[*selection_index].variant {
				JsonVariant::String(ref mut string) => {
					match input {
						JsonInput::Char(c) => {
							string.push(c);
						},
						JsonInput::Backspace => {
							string.pop();
						},
					}
				},
				JsonVariant::ObjectEntry(ref mut string, _) => {
					match input {
						JsonInput::Char(c) => {
							string.push(c);
						},
						JsonInput::Backspace => {
							string.pop();
						},
					}
				},
				_ => {
				},
			}
		}
	}
	pub fn objectify(&mut self) {
		for selection_index in self.selections.iter() {
			match self.nodes[*selection_index].variant {
				JsonVariant::Bool(_) | JsonVariant::String(_) | JsonVariant::Number(_) | JsonVariant::Null => {
					self.nodes[*selection_index].variant = JsonVariant::Object(Vec::new());
				},
				JsonVariant::ObjectEntry(_, _) | JsonVariant::Object(_) => {},
				JsonVariant::Array(ref children) => {
					let first_new_index = self.nodes.len();
					let last_new_index = first_new_index + children.len() - 1;
					let (new_children, mut new_nodes): (Vec<_>, Vec<_>) = children
						.iter()
						.enumerate()
						.map(|(id, child)| (id+first_new_index, child))
						.map(|(index, child)| {
							(index, JsonNode {
								variant: JsonVariant::ObjectEntry("".to_string(), *child),
								parent: *selection_index,
								left: if index==first_new_index {*selection_index} else {index-1},
								right: if index==last_new_index {*selection_index} else {index+1},
							})
						})
						.unzip();
					self.nodes[*selection_index].variant = JsonVariant::Object(new_children);
					self.nodes.append(&mut new_nodes);
				},
			}
		}
	}
	pub fn stringify(&mut self) {
		for selection_index in self.selections.iter() {
			match self.nodes[*selection_index].variant {
				JsonVariant::Null | JsonVariant::Array(_) | JsonVariant::Object(_) => {
					self.nodes[*selection_index].variant = JsonVariant::String("".to_string());
				},
				JsonVariant::Bool(b) => {
					self.nodes[*selection_index].variant = JsonVariant::String(b.to_string());
				},
				JsonVariant::Number(n) => {
					self.nodes[*selection_index].variant = JsonVariant::String(n.to_string());
				},
				JsonVariant::String(_) | JsonVariant::ObjectEntry(_, _) => {},
			}
		}
	}
}
