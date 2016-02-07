use std::rc::Rc;
use std::vec::Vec;


#[allow(dead_code)]  // lint bug
#[derive(Eq, PartialEq, Debug, Copy, Clone)]
pub enum CompileErrorType {
    LexerInvalidEscapeChar,
    LexerUnexpectedChar,
    LexerInCompleteString,
    LexerInvalidFloat,

    ParserNoMoreToken,
    ParserUnExpectedTokenType,
    ParserTableAttrNotExist,
    ParserNoTable,
    ParserLackOfSpecifyingTable,
    ParserCanNotParseLeftToken,
}

#[derive(Debug)]
pub struct CompileError {
    pub error_type : CompileErrorType,
    pub token : Rc<::parser::lexer::Token>,
    pub error_msg : String,
}

pub type ErrorRef = Rc<CompileError>;
pub type ErrorList = Vec<ErrorRef>;