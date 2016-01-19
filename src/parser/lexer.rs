use parser::combinators::{Parser, StrParser, ParseTo, Consumer, string, character};
use self::Token::{LParen, RParen, Whitespace, Identifier};

#[derive(Copy, Clone, Eq, Hash, Ord, PartialOrd, Debug)]
pub enum Token<'a> {
    LParen,
    RParen,
    Whitespace,
    Identifier(&'a str),
}

impl<'a,'b> PartialEq<Token<'b>> for Token<'a> {
    fn eq(&self, rhs: &Token<'b>) -> bool {
        match (*self, *rhs) {
            (LParen, LParen)                       => true,
            (RParen, RParen)                       => true,
            (Whitespace, Whitespace)               => true,
            (Identifier(ref x), Identifier(ref y)) => x == y,
            _                                      => false
        }
    }
}

impl<'a> From<Token<'a>> for String {
    fn from(tok: Token<'a>) -> String {
        String::from(match tok {
            LParen => "(",
            RParen => ")",
            Whitespace => "<space>",
            Identifier(x) => x,
        })
    }
}

pub trait LexerConsumer<D> where D: for<'a> Consumer<Token<'a>> {
    fn accept<L>(self, lexer: L) where L: for<'a> ParseTo<&'a str,D>;
}

fn mk_identifier<'a>(s: &'a str) -> Token<'a> { Identifier(s) }

#[allow(non_snake_case)]
pub fn lexer<C,D>(consumer: C) where C: LexerConsumer<D>, D: for<'a> Consumer<Token<'a>> {
    let LPAREN = string("(").map(|_| LParen);
    let RPAREN = string(")").map(|_| RParen);
    let WHITESPACE = character(char::is_whitespace).map(|_| Whitespace);
    let IDENTIFIER = character(char::is_alphabetic).and_then(character(char::is_alphanumeric).star())
                                                   .buffer().map(mk_identifier);
    let TOKEN = LPAREN.or_else(RPAREN).or_else(WHITESPACE).or_else(IDENTIFIER);
    consumer.accept(TOKEN.star())
}

#[test]
fn test_lexer() {
    struct TestConsumer(Vec<String>);
    impl<'a> Consumer<Token<'a>> for TestConsumer {
        fn accept(&mut self, tok: Token<'a>) {
            self.0.push(String::from(tok));
        }
    }
    impl LexerConsumer<TestConsumer> for TestConsumer {
        fn accept<L>(mut self, mut lex: L) where L: for<'a> ParseTo<&'a str,TestConsumer> {
            lex.push_to("(a123  bcd)", &mut self);
            assert_eq!(self.0, vec!["(", "a123", "<space>", "<space>", "bcd", ")"]);
        }
    }
    lexer(TestConsumer(Vec::new()));
}

#[test]
fn test_partial_eq() {
    use std::fmt::Debug;
    fn foo<T,U>(lhs: T, rhs: U) where T: Debug+PartialEq<U>, U: Debug{
        assert_eq!(lhs, rhs)
    }
    fn bar<'b,T>(lhs: T, rhs: Token<'b>) where T: Debug+for<'a> PartialEq<Token<'a>> {
        foo(lhs, rhs)
    }
    let hi = String::from("hi");
    bar(Identifier("hi"),Identifier(&*hi));
    bar(Identifier(&*hi),Identifier("hi"));
}
