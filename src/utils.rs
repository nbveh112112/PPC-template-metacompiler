use crate::tokenizer::{Token, TokenInfo};

pub(crate) fn skip_whitespace(tokens: &[TokenInfo], mut i: usize) -> usize {
    while i < tokens.len() {
        match &tokens[i].token {
            Token::Whitespace(_) | Token::Comment(_) => i += 1,
            _ => break,
        }
    }
    i
}

pub(crate) fn is_identifier(token: &Token) -> bool {
    matches!(token, Token::Identifier(_))
}

//skips whitespace and comments, then checks if the next token matches the expected token
pub(crate) fn check_next_token(tokens: &[TokenInfo], i: usize, expected: &Token) -> bool {
    let next_index = skip_whitespace(tokens, i);
    if next_index < tokens.len() {
        &tokens[next_index].token == expected
    } else {
        false
    }
}

pub(crate) fn skip_paren_seq(tokens: &[TokenInfo], mut i: usize, token_type : Token) -> usize {
    let mut paren_count = 0;
    let complement_token = match token_type {
        Token::LeftParen => Token::RightParen,
        Token::RightParen => Token::LeftParen,
        Token::Less => Token::Greater,
        _ => return i, // Not a parenthesis token, return the original index
    };
    while i < tokens.len() {
        if tokens[i].token == token_type {
            paren_count += 1;
        } else if tokens[i].token == complement_token {
            paren_count -= 1;
            if paren_count == 0 {
                return i + 1;
            }
        }
        i += 1;
    }
    i
}