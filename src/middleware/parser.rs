use std::cell::RefCell;

use html5ever::{
    tokenizer::{Tag, TagKind::StartTag, Token, TokenSink, TokenSinkResult},
    Attribute,
};

const BASE_URL: &str = "https://www.fia.com";

enum ParserState {
    None,
    BeginEvent,
    EventTitle,
    Document,
    DocumentTitle,
    DocumentDate,
    Next,
}

#[derive(Clone, Debug)]
pub struct ParserDocument {
    pub title: Option<String>,
    pub url: Option<String>,
    pub date: Option<String>,
}

#[derive(Debug)]
pub struct Season {
    pub year: i32,
    pub events: Vec<ParserEvent>,
}

#[derive(Debug)]
pub struct ParserEvent {
    pub title: Option<String>,
    pub season: Option<i32>,
    pub documents: Vec<ParserDocument>,
}

pub struct HTMLParser<'a> {
    state: RefCell<ParserState>,
    pub season: RefCell<Season>,
    event: RefCell<Option<ParserEvent>>,
    document: RefCell<Option<ParserDocument>>,
}

impl<'a> HTMLParser<'a> {
    pub fn new(
        season: RefCell<Season>,
        state: RefCell<ParserState>,
        event: RefCell<Option<ParserEvent>>,
        document: RefCell<Option<ParserDocument>>,
    ) -> Self {
        Self {
            state,
            season,
            event,
            document,
        }
    }
}

fn get_attr<'a>(
    tag: &Tag,
    name: &str,
) -> Option<Attribute> {
    let attr =
        tag.attrs.iter().cloned().find(|f| f.name.local.as_ref() == name);
    return attr;
}

impl<'a> TokenSink for HTMLParser<'a> {
    type Handle = ();

    fn process_token(
        &self,
        token: Token,
        _line_number: u64,
    ) -> TokenSinkResult<Self::Handle> {
        match token {
            Token::TagToken(tag_token) => {
                let name = tag_token.name.as_ref();
                let class = get_attr(&tag_token, "class");
                match (tag_token.kind, name) {
                    (StartTag, "ul") => {
                        if class.unwrap().value.as_ref() == "event-wrapper" {
                            *self.state.get_mut() = ParserState::BeginEvent;
                        }
                    },
                    (StartTag, "a") => {
                        match self.state.get_mut() {
                            ParserState::Next => {},
                            _ => {
                                return TokenSinkResult::Continue;
                            },
                        }
                        if let Some(href) =
                            get_attr(&tag_token, "href").as_ref()
                        {
                            let href = href.value.as_ref();
                            *self.document.get_mut() = Some(ParserDocument {
                                url: Some(format!(
                                    "{}{}",
                                    BASE_URL,
                                    href.trim().replace(' ', "%20")
                                )),
                                title: None,
                                date: None,
                            });
                            *self.state.get_mut() = ParserState::Document;
                        }
                    },
                    (StartTag, "div") => {
                        if class.is_none() {
                            return TokenSinkResult::Continue;
                        }
                        let class = class.as_ref().unwrap().value.as_ref();
                        match self.state.get_mut() {
                            ParserState::BeginEvent => {
                                if class.starts_with("event-title") {
                                    *self.state.get_mut() =
                                        ParserState::EventTitle;
                                }
                            },
                            ParserState::Document => {
                                if class == "title" {
                                    *self.state.get_mut() =
                                        ParserState::DocumentTitle;
                                }
                            },
                            _ => return TokenSinkResult::Continue,
                        }
                    },
                    (StartTag, "span") => match self.state.get_mut() {
                        ParserState::Document => {
                            if class.as_ref().unwrap().value.as_ref()
                                == "date-display-single"
                            {
                                *self.state.get_mut() =
                                    ParserState::DocumentDate;
                            }
                        },
                        _ => {},
                    },

                    _ => {},
                }
            },
            Token::CharacterTokens(chars) => match self.state.get_mut() {
                ParserState::EventTitle => {
                    if chars.trim().len() == 0 {
                        return TokenSinkResult::Continue;
                    }
                    if let Some(event) = self.event.take() {
                        self.season.get_mut().events.push(event);
                    }
                    let event = ParserEvent {
                        season: Some(self.season.get_mut().year),
                        title: Some(chars.trim().to_owned()),
                        documents: Vec::with_capacity(60),
                    };
                    *self.state.get_mut() = ParserState::Next;
                    *self.event.get_mut() = Some(event);
                },
                ParserState::DocumentTitle => {
                    if chars.trim().len() == 0 {
                        return TokenSinkResult::Continue;
                    }
                    self.document.get_mut().unwrap().title =
                        Some(chars.trim().to_owned());
                    *self.state.get_mut() = ParserState::Document;
                },
                ParserState::DocumentDate => {
                    if chars.trim().len() == 0 {
                        return TokenSinkResult::Continue;
                    }
                    *self.document.get_mut().as_ref().unwrap().date =
                        Some(chars.trim().to_owned());
                    *self.state.get_mut() = ParserState::Next;
                    if let Some(doc) = self.document.take() {
                        self.event.get_mut().unwrap().documents.push(doc);
                    }
                },
                ParserState::Document => {},
                _ => {},
            },
            Token::EOFToken => {
                if let Some(event) = self.event.take() {
                    self.season.get_mut().events.push(event);
                }
            },
            _ => {},
        }
        return TokenSinkResult::Continue;
    }
}
