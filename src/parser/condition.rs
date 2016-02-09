use std::fmt;
use std::fmt::{Formatter, Display};
use std::rc::Rc;
use std::result::Result::{Ok, Err};
use super::common::{ValueType, ValueExpr};
use super::lexer::{TokenIter, TokenType};
use super::compile_error::{CompileError, CompileErrorType, ErrorList};
use super::attribute::AttributeExpr;
use super::common::{
    align_iter,
    get_next_token,
    consume_next_token,
    consume_next_token_with_type,
    consume_next_token_with_type_list,
};


#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum LogicOp {
    Or,
    And,
}

impl Display for LogicOp {
    fn fmt(&self, f : &mut Formatter) -> fmt::Result {
        write!(f, "{}", match self {
            &LogicOp::Or => "or".to_string(),
            &LogicOp::And => "and".to_string(),
        })
    }
}


#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum CmpOp {
    LT,
    GT,
    LE,
    GE,
    EQ,
    NE,
    Is,
    IsNot,
}

impl Display for CmpOp {
    fn fmt(&self, f : &mut Formatter) -> fmt::Result {
        write!(f, "{}", match self {
            &CmpOp::LT => "<".to_string(),
            &CmpOp::GT => ">".to_string(),
            &CmpOp::LE => "<=".to_string(),
            &CmpOp::GE => ">=".to_string(),
            &CmpOp::EQ => "=".to_string(),
            &CmpOp::NE => "!=".to_string(),
            &CmpOp::Is => "is".to_string(),
            &CmpOp::IsNot => "is not".to_string(),
        })
    }
}


#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum ArithOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

impl Display for ArithOp {
    fn fmt(&self, f : &mut Formatter) -> fmt::Result {
        write!(f, "{}", match self {
            &ArithOp::Add => "+".to_string(),
            &ArithOp::Sub => "-".to_string(),
            &ArithOp::Mul => "*".to_string(),
            &ArithOp::Div => "/".to_string(),
            &ArithOp::Mod => "%".to_string(),
        })
    }
}


trait Expr : Display + ToString {}


fn binary_fmt<T, U>(operator : U, lhs : &T, rhs : &T, f : &mut Formatter) -> fmt::Result
    where T : Display, U : Display {
    write!(f, "({} {} {})", lhs, operator, rhs)
}

fn unary_fmt<T>(operator: &str, operant : &T, f : &mut Formatter) -> fmt::Result
    where T : Display {
    write!(f, "({} {})", operator, operant)
}

type CondRef = Box<ConditionExpr>;
pub type ParseCondResult = Result<ConditionExpr, ErrorList>;

#[derive(Debug)]
pub enum ConditionExpr {
    LogicExpr {
        lhs : CondRef,
        rhs :CondRef,
        op : LogicOp,
    },
    NotExpr { operant : CondRef },
    CmpExpr {
        lhs : CmpOperantExpr,
        rhs : CmpOperantExpr,
        op : CmpOp,
    },
}

impl Display for ConditionExpr {
    fn fmt(&self, f : &mut Formatter) -> fmt::Result {
        match self {
            &ConditionExpr::LogicExpr{ref lhs, ref rhs, op} => binary_fmt(op, lhs, rhs, f),
            &ConditionExpr::NotExpr{ref operant} => unary_fmt("not", operant, f),
            &ConditionExpr::CmpExpr{ref lhs, ref rhs, op} => binary_fmt(op, lhs, rhs, f),
        }
    }
}


type CmpOperantRef = Box<CmpOperantExpr>;
pub type ParseCmpOperantResult = Result<CmpOperantExpr, ErrorList>;

#[derive(Debug)]
pub enum CmpOperantExpr {
    Arith(ArithExpr),
    Value(ValueExpr),
}

impl Display for CmpOperantExpr {
    fn fmt(&self, f : &mut Formatter) -> fmt::Result {
        match self {
            &CmpOperantExpr::Arith(ref arith_exp) => arith_exp.fmt(f),
            &CmpOperantExpr::Value(ValueExpr{ref value, value_type}) => write!(f, "{:?}({})", value_type, value),
        }
    }
}


pub type ArithRef = Box<ArithExpr>;
pub type ParseArithResult = Result<ArithExpr, ErrorList>;

#[derive(Debug)]
pub enum ArithExpr {
    BinaryExpr {
        lhs : ArithRef,
        rhs : ArithRef,
        op : ArithOp,
    },
    MinusExpr { operant : ArithRef },
    Value(ValueExpr),
    Attr(AttributeExpr),
}

impl Display for ArithExpr {
    fn fmt(&self, f : &mut Formatter) -> fmt::Result {
        match self {
            &ArithExpr::BinaryExpr{ref lhs, ref rhs, op} => binary_fmt(op, lhs, rhs, f),
            &ArithExpr::MinusExpr{ref operant} => unary_fmt("-", operant, f),
            &ArithExpr::Value(ValueExpr{ref value, value_type}) => write!(f, "{:?}({})", value_type, value),
            &ArithExpr::Attr(ref attribute) => attribute.fmt(f),
        }
    }
}

macro_rules! parse_binary {
    ($iter:expr, $ops:expr, $expr_type:ident :: $sub_parse_func:ident, $binary_pat:ident, $exp_ref:ident, $to_op:ident) => ({
        let mut exp = try!($expr_type::$sub_parse_func($iter));
        loop {
            let mut tmp = $iter.clone();
            let token = match consume_next_token(&mut tmp) {
                Ok(token) => token,
                Err(..) => return Ok(exp),
            };
            if !$ops.contains(&token.token_type) {
                return Ok(exp);
            }
            let rhs = match $expr_type::$sub_parse_func(&mut tmp) {
                Ok(exp) => exp,
                Err(..) => return Ok(exp),
            };
            align_iter($iter, &mut tmp);
            let binary_exp = $expr_type::$binary_pat{
                lhs : $exp_ref::new(exp),
                rhs : $exp_ref::new(rhs),
                op : $to_op(token.token_type),
            };
            exp = binary_exp;
        }
    });
}

impl ConditionExpr {
    pub fn parse(it : &mut TokenIter) -> ParseCondResult {
        ConditionExpr::parse_or(it)
    }

    pub fn parse_or(it : &mut TokenIter) -> ParseCondResult {
        let ops = [TokenType::Or];
        parse_binary!(it, ops, ConditionExpr::parse_and, LogicExpr, CondRef, to_logic_op)
    }

    pub fn parse_and(it : &mut TokenIter) -> ParseCondResult {
        let ops = [TokenType::And];
        parse_binary!(it, ops, ConditionExpr::parse_primitive, LogicExpr, CondRef, to_logic_op)
    }

    pub fn parse_primitive(it : &mut TokenIter) -> ParseCondResult {
        let token = try!(get_next_token(it));
        match token.token_type {
            TokenType::Not => {
                it.next();
                Ok(ConditionExpr::NotExpr { operant : CondRef::new(try!(ConditionExpr::parse(it))) })
            }
            TokenType::OpenBracket => {
                it.next();
                let cond_exp = try!(ConditionExpr::parse(it));
                try!(consume_next_token_with_type(it, TokenType::CloseBracket));
                Ok(cond_exp)
            }
            _ => Ok(try!(ConditionExpr::parse_cmp(it))),
        }
    }

    pub fn parse_cmp(it : &mut TokenIter) -> ParseCondResult {
        let ops = vec![
            TokenType::LT,
            TokenType::GT,
            TokenType::LE,
            TokenType::GE,
            TokenType::EQ,
            TokenType::NE,
            TokenType::Is,
            TokenType::IsNot,
        ];
        let lhs = try!(CmpOperantExpr::parse(it));
        let token = try!(consume_next_token_with_type_list(it, &ops));
        let rhs = try!(CmpOperantExpr::parse(it));
        Ok(ConditionExpr::CmpExpr{
            lhs : lhs,
            rhs : rhs,
            op : to_cmp_op(token.token_type),
        })
    }
}

impl CmpOperantExpr {
    pub fn parse(it : &mut TokenIter) -> ParseCmpOperantResult {
        let token = try!(get_next_token(it));
        match token.token_type {
            TokenType::StringLiteral | TokenType::Null => {
                it.next();
                Ok(CmpOperantExpr::Value(ValueExpr{
                    value : token.value.clone(),
                    value_type : token_type_to_value_type(token.token_type),
                }))
            }
            _ => Ok(CmpOperantExpr::Arith(try!(ArithExpr::parse(it))))
        }
    }
}

impl ArithExpr {
    pub fn parse(it : &mut TokenIter) -> ParseArithResult {
        ArithExpr::parse_first_binary(it)
    }

    pub fn parse_first_binary(it : &mut TokenIter) -> ParseArithResult {
        let ops = [TokenType::Add, TokenType::Sub];
        parse_binary!(it, ops, ArithExpr::parse_second_binary, BinaryExpr, ArithRef, to_arith_op)
    }

    pub fn parse_second_binary(it : &mut TokenIter) -> ParseArithResult {
        let ops = [TokenType::Star, TokenType::Div, TokenType::Mod];
        parse_binary!(it, ops, ArithExpr::parse_primitive, BinaryExpr, ArithRef, to_arith_op)
    }

    pub fn parse_primitive(it : &mut TokenIter) -> ParseArithResult {
        let token = try!(get_next_token(it));
        match token.token_type {
            TokenType::Sub => {
                it.next();
                Ok(ArithExpr::MinusExpr{ operant : ArithRef::new(try!(ArithExpr::parse(it))) })
            }
            TokenType::Add => {
                it.next();
                Ok(try!(ArithExpr::parse(it)))
            }
            TokenType::OpenBracket => {
                it.next();
                let arith_exp = try!(ArithExpr::parse(it));
                try!(consume_next_token_with_type(it, TokenType::CloseBracket));
                Ok(arith_exp)
            }
            _ => Ok(try!(ArithExpr::parse_arith_operant(it))),
        }
    }

    pub fn parse_arith_operant(it : &mut TokenIter) -> ParseArithResult {
        let token = try!(get_next_token(it));
        match token.token_type {
            TokenType::IntegerLiteral | TokenType::FloatLiteral => {
                it.next();
                Ok(ArithExpr::Value(ValueExpr{
                    value : token.value.clone(),
                    value_type : token_type_to_value_type(token.token_type),
                }))
            }
            TokenType::Identifier => {
                Ok(ArithExpr::Attr(try!(AttributeExpr::parse(it))))
            }
            _ => {
                let err_msg = format!("unexpected tokentype: {:?}, expect Literal or Identifier",
                    token.token_type);
                let e = Rc::new(CompileError{
                    error_type : CompileErrorType::ParserUnExpectedTokenType,
                    token : token,
                    error_msg : err_msg,
                });
                Err(vec![e])
            }
        }
    }
}

fn token_type_to_value_type(t : TokenType) -> ValueType {
    match t {
        TokenType::IntegerLiteral => ValueType::Integer,
        TokenType::FloatLiteral => ValueType::Float,
        TokenType::StringLiteral => ValueType::String,
        TokenType::Null => ValueType::Null,
        _ => panic!("unexpected TokenType: {:?}", t),
    }
}

fn to_arith_op(token_type : TokenType) -> ArithOp {
    match token_type {
        TokenType::Add => ArithOp::Add,
        TokenType::Sub => ArithOp::Sub,
        TokenType::Star => ArithOp::Mul,
        TokenType::Div => ArithOp::Div,
        TokenType::Mod => ArithOp::Mod,
        _ => panic!("unexpected token type: {:?}", token_type),
    }
}

fn to_logic_op(token_type : TokenType) -> LogicOp {
    match token_type {
        TokenType::Or => LogicOp::Or,
        TokenType::And => LogicOp::And,
        _ => panic!("unexpected token type: {:?}", token_type),
    }
}

fn to_cmp_op(token_type : TokenType) -> CmpOp {
    match token_type {
        TokenType::LT => CmpOp::LT,
        TokenType::GT => CmpOp::GT,
        TokenType::LE => CmpOp::LE,
        TokenType::GE => CmpOp::GE,
        TokenType::EQ => CmpOp::EQ,
        TokenType::NE => CmpOp::NE,
        TokenType::Is => CmpOp::Is,
        TokenType::IsNot => CmpOp::IsNot,
        _ => panic!("unexpected token type: {:?}", token_type),
    }
}
