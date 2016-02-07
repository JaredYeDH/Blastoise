use std::rc::Rc;
use std::result::Result::{Ok, Err};
use std::iter::ExactSizeIterator;
use super::lexer::{Token, TokenRef, TokenType, TokenIter};
use super::compile_error::{CompileError, CompileErrorType, ErrorRef, ErrorList};


#[allow(dead_code)]  // lint bug
#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum ValueType {
    Integer,
    Float,
    String,
    Null,
}

fn gen_end_token(it : &TokenIter) -> TokenRef {
    let mut it = it.clone();
    match it.next_back() {
        Some(token_ref) => token_ref.clone(),
        // dummy token
        None => Rc::new(Token{
            column : 0,
            value : "".to_string(),
            token_type : TokenType::UnKnown
        }),
    }
}

fn gen_reach_the_end_err_with_type(it : &TokenIter, expected_token_type : TokenType) -> ErrorList {
    let token = gen_end_token(it);
    let err_msg = format!("expect {:?} but no more token found", expected_token_type);
    let err = Rc::new(CompileError{
        error_type : CompileErrorType::ParserNoMoreToken,
        token : token,
        error_msg : err_msg,
    });
    vec![err]
}

fn gen_reach_the_end_err(it : &TokenIter) -> ErrorList {
    let token = gen_end_token(it);
    let error_msg = format!("expect token but no more token found");
    let err = Rc::new(CompileError{
        error_type : CompileErrorType::ParserNoMoreToken,
        token : token,
        error_msg : error_msg,
    });
    vec![err]
}

pub fn consume_next_token_with_type(it : &mut TokenIter, token_type : TokenType)
        -> Result<TokenRef, ErrorList> {
    let token : TokenRef = match it.next() {
        Some(token) => (*token).clone(),
        None => return Err(gen_reach_the_end_err_with_type(it, token_type)),
    };
    if token_type == token.token_type {
        return Ok(token);
    }
    let err_msg = format!("expect token type: {:?}, but got {:?}",
        token_type, token.token_type);
    let err = Rc::new(CompileError{
        error_type : CompileErrorType::ParserUnExpectedTokenType,
        token : token.clone(),
        error_msg : err_msg,
    });
    Err(vec![err])
}

pub fn get_next_token(it : &TokenIter) -> Result<TokenRef, ErrorList> {
    match it.clone().peekable().peek() {
        Some(token) => Ok((*token).clone()),
        None => Err(gen_reach_the_end_err(it)),
    }
}

pub fn consume_next_token(it : &mut TokenIter)
        -> Result<TokenRef, ErrorList> {
    match it.next() {
        Some(token) => Ok(token.clone()),
        None => Err(gen_reach_the_end_err(it)),
    }
}

pub fn check_parse_to_end(it : &TokenIter) -> Option<ErrorRef> {
    match it.clone().peekable().peek() {
        None => None,
        Some(token) => Some(Rc::new(CompileError{
            error_type : CompileErrorType::ParserCanNotParseLeftToken,
            token : (*token).clone(),
            error_msg : format!("Can not parse the left tokens : {:?}", token.value),
        })),
    }
}

pub fn align_iter(it1 : &mut TokenIter, it2 : &mut TokenIter) {
    let (l1, l2) = (it1.len(), it2.len());
    if l1 < l2 {
        return align_iter(it2, it1);
    }
    if l1 - l2 > 0 {
        it1.nth(l1 - l2 - 1);
    }
}

macro_rules! try_parse_helper {
    ($result:expr, $iter:expr, $tmp:expr) => ({
        use std::result::Result::{Ok, Err};
        use std::convert::From;
        match $result {
            Ok(val) => {
                ::parser::common::align_iter($iter, &mut $tmp);
                val
            },
            Err(err) => {
                return Err(From::from(err))
            },
        }
    });
}

// when success, iter will be changed
// while not, iter stay still
macro_rules! try_parse {
    ($parse_func:ident, $iter:expr) => ({
        let mut tmp = $iter.clone();
        try_parse_helper!($parse_func(&mut tmp), $iter, tmp)
    });
    ($parse_func:ident, $iter:expr, $( $add_args:expr ),*) => ({
        let mut tmp = $iter.clone();
        try_parse_helper!($parse_func(&mut tmp, $($add_args),* ), $iter, tmp)
    });
    ($type_name:ident :: $parse_func:ident, $iter:expr) => ({
        let mut tmp = $iter.clone();
        try_parse_helper!($type_name::$parse_func(&mut tmp), $iter, tmp)
    });
    ($type_name:ident :: $parse_func:ident, $iter:expr, $( $add_args:expr ),*) => ({
        let mut tmp = $iter.clone();
        try_parse_helper!($type_name::$parse_func(&mut tmp, $($add_args),* ), $iter, tmp)
    });
}

macro_rules! or_parse_helper {
    ($result:expr, $iter:expr, $tmp:expr) => ({
        use std::result::Result::{Ok, Err};
        match $result {
            Ok(val) => {
                ::parser::common::align_iter($iter, &mut $tmp);
                return Ok(val)
            },
            Err(err) => {
                err
            },
        }
    });
}

// when success, iter will be changed and return the parsed Expr
// while not, iter stay still
macro_rules! or_parse {
    ($parse_func:ident, $iter:expr) => ({
        let mut tmp = $iter.clone();
        or_parse_helper!($parse_func(&mut tmp), $iter, tmp)
    });
    ($parse_func:ident, $iter:expr, $( $add_args:expr ),*) => ({
        let mut tmp = $iter.clone();
        or_parse_helper!($parse_func(&mut tmp, $($add_args),* ), $iter, tmp)
    });
    ($type_name:ident :: $parse_func:ident, $iter:expr) => ({
        let mut tmp = $iter.clone();
        or_parse_helper!($type_name::$parse_func(&mut tmp), $iter, tmp)
    });
    ($type_name:ident :: $parse_func:ident, $iter:expr, $( $add_args:expr ),*) => ({
        let mut tmp = $iter.clone();
        or_parse_helper!($type_name::$parse_func(&mut tmp, $($add_args),* ), $iter, tmp)
    });
}

macro_rules! or_parse_combine {
    ($iter:expr, $( $type_name:ident :: $parse_func:ident ),+ ) => ({
        use std::vec::Vec;
        let mut errors = Vec::new();
        $(
            let errs = or_parse!($type_name::$parse_func, $iter);
            ::parser::common::extend_from_other(&mut errors, &errs);
        )+
        Err(errors)
    });
}

// should be replaced when upgrade rust to 1.6
pub fn extend_from_other(errors : &mut Vec<ErrorRef>, other : &Vec<ErrorRef>) {
    errors.extend(other.iter().cloned());
}