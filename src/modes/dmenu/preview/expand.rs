//! Shell-aware expansion of trusted preview command placeholders.

use eyre::Result;

pub(super) fn expand_preview_command(template: &str) -> Result<String, String> {
    let mut command = String::with_capacity(template.len() + 16);
    let mut remaining = template;
    let mut quote = ShellQuote::Unquoted;
    let mut escaped = false;
    let mut substitutions = Vec::<CommandSubstitution>::new();

    while !remaining.is_empty() {
        let arithmetic_context = substitutions
            .last()
            .is_some_and(|substitution| substitution.arithmetic);
        if !escaped
            && quote == ShellQuote::Unquoted
            && !arithmetic_context
            && remaining.starts_with("<<")
        {
            return Err("Preview command heredocs are not supported".to_string());
        } else if !escaped && let Some(rest) = remaining.strip_prefix("{}") {
            append_placeholder(&mut command, quote, "FSEL_PREVIEW_ITEM")?;
            remaining = rest;
        } else if !escaped && let Some(rest) = remaining.strip_prefix("{q}") {
            append_placeholder(&mut command, quote, "FSEL_PREVIEW_QUERY")?;
            remaining = rest;
        } else if !escaped && let Some(rest) = remaining.strip_prefix("{n}") {
            if arithmetic_context {
                command.push_str("FSEL_PREVIEW_ORDINAL");
            } else {
                append_placeholder(&mut command, quote, "FSEL_PREVIEW_ORDINAL")?;
            }
            remaining = rest;
        } else if !escaped
            && quote != ShellQuote::Single
            && let Some(rest) = remaining.strip_prefix("$(")
        {
            command.push_str("$(");
            if let Some(parent) = substitutions.last_mut() {
                parent.command_position = false;
            }
            substitutions.push(CommandSubstitution {
                outer_quote: quote,
                nested_parentheses: 0,
                case_patterns: Vec::new(),
                arithmetic: remaining.starts_with("$(("),
                command_position: true,
                delimiter: SubstitutionDelimiter::Parenthesis,
            });
            quote = ShellQuote::Unquoted;
            remaining = rest;
        } else if !escaped
            && quote != ShellQuote::Single
            && let Some(rest) = remaining.strip_prefix('`')
        {
            command.push('`');
            if substitutions.last().is_some_and(|substitution| {
                substitution.delimiter == SubstitutionDelimiter::Backtick
            }) {
                quote = substitutions
                    .pop()
                    .expect("substitution stack is non-empty")
                    .outer_quote;
            } else {
                if let Some(parent) = substitutions.last_mut() {
                    parent.command_position = false;
                }
                substitutions.push(CommandSubstitution {
                    outer_quote: quote,
                    nested_parentheses: 0,
                    case_patterns: Vec::new(),
                    arithmetic: false,
                    command_position: true,
                    delimiter: SubstitutionDelimiter::Backtick,
                });
                quote = ShellQuote::Unquoted;
            }
            remaining = rest;
        } else if !escaped
            && quote == ShellQuote::Unquoted
            && substitutions.last().is_some_and(|substitution| {
                substitution.command_position && !substitution.arithmetic
            })
            && let Some(keyword) = shell_control_keyword(template, remaining)
        {
            command.push_str(keyword);
            remaining = &remaining[keyword.len()..];
            let substitution = substitutions
                .last_mut()
                .expect("substitution stack is non-empty");
            match keyword {
                "case" => {
                    substitution.case_patterns.push(true);
                    substitution.command_position = false;
                }
                "esac" => {
                    substitution.case_patterns.pop();
                    substitution.command_position = false;
                }
                "then" | "do" | "else" | "elif" => {
                    substitution.command_position = true;
                }
                _ => unreachable!("all recognized control keywords are handled"),
            }
        } else {
            let Some(character) = remaining.chars().next() else {
                break;
            };
            command.push(character);
            remaining = &remaining[character.len_utf8()..];
            if escaped {
                escaped = false;
                continue;
            }
            match (quote, character) {
                (ShellQuote::Unquoted, '\\') | (ShellQuote::Double, '\\') => escaped = true,
                (ShellQuote::Unquoted, '\'') => quote = ShellQuote::Single,
                (ShellQuote::Unquoted, '"') => quote = ShellQuote::Double,
                (ShellQuote::Single, '\'') | (ShellQuote::Double, '"') => {
                    quote = ShellQuote::Unquoted;
                }
                (ShellQuote::Unquoted, '(') if !substitutions.is_empty() => {
                    let substitution = substitutions
                        .last_mut()
                        .expect("substitution stack is non-empty");
                    let starts_case_pattern = command[..command.len() - 1]
                        .chars()
                        .next_back()
                        .is_none_or(|previous| previous.is_whitespace() || previous == '|');
                    // An optional leading parenthesis is part of a case pattern.
                    // Parentheses within the pattern (for example, extglobs) still
                    // need balancing before the command substitution can close.
                    if substitution.case_patterns.last() != Some(&true) || !starts_case_pattern {
                        substitution.nested_parentheses += 1;
                    }
                }
                (ShellQuote::Unquoted, ')') if !substitutions.is_empty() => {
                    let substitution = substitutions
                        .last_mut()
                        .expect("substitution stack is non-empty");
                    if substitution.case_patterns.last() == Some(&true) {
                        // A case pattern terminator belongs to the case grammar,
                        // not to the surrounding command substitution.
                        if let Some(in_pattern) = substitution.case_patterns.last_mut() {
                            *in_pattern = false;
                        }
                        substitution.command_position = true;
                        continue;
                    } else if substitution.nested_parentheses == 0
                        && substitution.delimiter == SubstitutionDelimiter::Parenthesis
                    {
                        quote = substitutions
                            .pop()
                            .expect("substitution stack is non-empty")
                            .outer_quote;
                    } else if substitution.nested_parentheses > 0 {
                        substitution.nested_parentheses -= 1;
                    }
                }
                _ => {}
            }
            if quote == ShellQuote::Unquoted
                && let Some(substitution) = substitutions.last_mut()
            {
                match character {
                    ';' | '|' | '&' | '\n' => {
                        substitution.command_position = true;
                        if character == ';'
                            && remaining.starts_with(';')
                            && let Some(in_pattern) = substitution.case_patterns.last_mut()
                        {
                            *in_pattern = true;
                        }
                    }
                    '(' | '{' if substitution.command_position => {
                        substitution.command_position = true;
                    }
                    character if character.is_whitespace() => {}
                    _ => substitution.command_position = false,
                }
            }
        }
    }

    Ok(command)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ShellQuote {
    Unquoted,
    Single,
    Double,
}

struct CommandSubstitution {
    outer_quote: ShellQuote,
    nested_parentheses: usize,
    case_patterns: Vec<bool>,
    arithmetic: bool,
    command_position: bool,
    delimiter: SubstitutionDelimiter,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SubstitutionDelimiter {
    Parenthesis,
    Backtick,
}

fn shell_control_keyword(template: &str, remaining: &str) -> Option<&'static str> {
    ["case", "esac", "then", "do", "else", "elif"]
        .into_iter()
        .find(|keyword| starts_shell_keyword(template, remaining, keyword))
}

fn starts_shell_keyword(template: &str, remaining: &str, keyword: &str) -> bool {
    let Some(after_keyword) = remaining.strip_prefix(keyword) else {
        return false;
    };
    let offset = template.len().saturating_sub(remaining.len());
    let previous = template[..offset].chars().next_back();
    let next = after_keyword.chars().next();
    previous.is_none_or(is_shell_word_boundary) && next.is_none_or(is_shell_word_boundary)
}

fn is_shell_word_boundary(character: char) -> bool {
    !character.is_alphanumeric() && character != '_'
}

fn append_placeholder(
    command: &mut String,
    quote: ShellQuote,
    variable: &str,
) -> Result<(), String> {
    match quote {
        ShellQuote::Unquoted => command.push_str(&format!("\"${variable}\"")),
        // Empty adjacent quotes terminate the variable name without changing the
        // surrounding double-quoted context. This works in POSIX shells and fish.
        ShellQuote::Double => command.push_str(&format!("${variable}\"\"")),
        ShellQuote::Single => {
            return Err("Preview placeholders must not appear inside single quotes".to_string());
        }
    }
    Ok(())
}
