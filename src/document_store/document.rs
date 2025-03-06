use std::str::FromStr;

use lsp_types::{Position, Uri};

use crate::parser::php::PhpParser;
use crate::parser::tokens::Token;
use crate::parser::yaml::YamlParser;

#[derive(Debug, PartialEq)]
pub enum FileType {
    Php,
    Yaml,
    Unknown,
}

#[derive(Debug)]
pub struct Document {
    pub file_type: FileType,
    pub content: String,
    pub tokens: Vec<Token>,
    uri: String,
}

impl Document {
    pub fn new(uri: &String, content: String) -> Self {
        Self {
            file_type: uri_to_file_type(uri),
            uri: uri.to_string(),
            tokens: vec![],
            content,
        }
    }

    pub fn set_content(&mut self, content: String) {
        self.content = content;
    }

    pub fn parse(&mut self) {
        self.tokens = match self.file_type {
            FileType::Php => {
                let parser = PhpParser::new(&self.content);
                parser.get_tokens()
            }
            FileType::Yaml => {
                let parser = YamlParser::new(&self.content, &self.uri);
                parser.get_tokens()
            }
            FileType::Unknown => {
                log::error!("Unable to parse documet {:?}", self);
                vec![]
            }
        };
    }

    pub fn get_uri(&self) -> Option<Uri> {
        Uri::from_str(&self.uri).ok()
    }

    pub fn get_token_under_cursor(&self, position: Position) -> Option<Token> {
        match self.file_type {
            FileType::Php => {
                let parser = PhpParser::new(&self.content);
                parser.get_token_at_position(position)
            }
            FileType::Yaml => {
                let parser = YamlParser::new(&self.content, &self.uri);
                parser.get_token_at_position(position)
            }
            _ => None,
        }
    }
}

fn uri_to_file_type(uri: &str) -> FileType {
    if uri.ends_with(".yml") || uri.ends_with(".yaml") {
        FileType::Yaml
    } else if uri.ends_with(".php")
        || uri.ends_with(".module")
        || uri.ends_with(".theme")
        || uri.ends_with(".install")
    {
        FileType::Php
    } else {
        FileType::Unknown
    }
}

#[cfg(test)]
mod tests {
    use crate::document_store::document::{Document, FileType};

    #[test]
    fn uri_to_file_type() {
        let document = Document::new(&String::from("file://test.php"), String::new());
        assert_eq!(FileType::Php, document.file_type);

        let document = Document::new(&String::from("file://test.yml"), String::new());
        assert_eq!(FileType::Yaml, document.file_type);

        let document = Document::new(&String::from("file://test.yaml"), String::new());
        assert_eq!(FileType::Yaml, document.file_type);

        let document = Document::new(&String::from("file://test"), String::new());
        assert_eq!(FileType::Unknown, document.file_type);

        let document = Document::new(&String::from("file://test.php.txt"), String::new());
        assert_eq!(FileType::Unknown, document.file_type);
    }
}
