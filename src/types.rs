#[derive(Clone, PartialEq)]
pub struct CharState {
    pub char_index: usize,
    pub reference_char: char,
    pub typed_char: Option<char>,
}

impl CharState {
    fn new(char_index: usize, reference_char: char) -> Self {
        CharState {
            char_index,
            reference_char,
            typed_char: None,
        }
    }
    pub fn typed(&mut self, typed_char: char) {
        self.typed_char = Some(typed_char);
    }
    pub fn backspace(&mut self) {
        self.typed_char = None
    }
}
#[derive(Clone)]
pub struct WordState {
    pub char_index: usize,
    pub data: Vec<CharState>,
}
#[derive(Clone)]
pub struct TypeState {
    pub word_index: usize,
    pub data: Vec<WordState>,
    pub focus: bool,
}

impl TypeState {
    pub fn from_str(value: &String) -> Self {
        TypeState {
            word_index: 0,
            data: value
                .split(' ')
                .map(|part| WordState {
                    char_index: 0,
                    data: part
                        .chars()
                        .enumerate()
                        .map(|(index, reference_char)| CharState::new(index, reference_char))
                        .collect(),
                })
                .collect(),
            focus: false,
        }
    }
}
