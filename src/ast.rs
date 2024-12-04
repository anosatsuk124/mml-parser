use core::panic;

use anyhow::Result;
use pest::{iterators::Pair, Parser};
use pest_derive::Parser;

#[derive(Parser, Debug, Clone)]
#[grammar = "./grammar/mml.pest"]
pub struct MmlParser;

#[derive(Debug, Clone)]
pub enum MmlAst {
    Note {
        note: char,
        length: Option<isize>,
        gate: Option<isize>,
        velocity: Option<isize>,
        timing: Option<isize>,
        scale: Option<isize>,
    },
    NNote {
        note_no: isize,
        length: Option<isize>,
        gate: Option<isize>,
        velocity: Option<isize>,
        timing: Option<isize>,
    },
    Rest(Option<isize>),
    Length(isize),
    Octave(isize),
    PitchBend(isize),
    Gate(isize),
    Velocity {
        value: isize,
        random: Option<isize>,
    },
    Timing {
        value: isize,
        random: Option<isize>,
    },
    ControlChange {
        controller: isize,
        value: isize,
        on_time: Option<(isize, isize, isize)>,
    },
    Macro(String),
    VoiceSelect {
        number: isize,
        bank_lsb: Option<isize>,
        bank_msb: Option<isize>,
    },
    OctaveUp,
    OctaveDown,
    VelocityUp(Option<isize>),
    VelocityDown(Option<isize>),
    Comment {
        kind: CommentKind,
        content: String,
    },
    LoopBegin(Option<isize>),
    LoopBreak,
    LoopEnd,
    Harmony {
        notes: Vec<char>,
        length: Option<isize>,
        gate: Option<isize>,
    },
    RhythmMacroDefine {
        name: char,
        definition: Box<MmlAst>,
    },
    GroupedNotes {
        notes: Vec<MmlAst>,
        length: Option<isize>,
    },
    OctaveUpOnce,
    OctaveDownOnce,
    PlayFromHere,
    TieSlur,
}

#[derive(Debug, Clone)]
pub enum CommentKind {
    RangeComment,
    LineCommentDebug,
    LineComment,
}

impl MmlAst {
    pub fn parse(input: &str) -> Result<Vec<MmlAst>> {
        let parsed = MmlParser::parse(Rule::mml, input)?;
        let mut ast: Vec<MmlAst> = Vec::new();
        for pair in parsed {
            for inner_pair in pair.into_inner() {
                let node = MmlAst::parse_command(inner_pair)?;
                if let Some(node) = node {
                    ast.push(node);
                }
            }
        }
        Ok(ast)
    }

    fn parse_command(pair: Pair<Rule>) -> Result<Option<MmlAst>> {
        let ast = match pair.as_rule() {
            Rule::abc_note => {
                let note_char = pair.clone().as_str().chars().next().unwrap();

                let mut inner = pair.into_inner();

                let length = match inner.next() {
                    Some(pair) => Some(extract_number(pair)?),
                    None => None,
                };

                let mut params = [None; 4];

                for (i, pair) in inner.enumerate() {
                    if let Ok(num) = extract_number(pair) {
                        params[i] = Some(num);
                    }
                }

                Ok(MmlAst::Note {
                    note: note_char,
                    length,
                    gate: params[0],
                    velocity: params[1],
                    timing: params[2],
                    scale: params[3],
                })
            }
            Rule::midi_note => {
                let mut inner_rules = pair.into_inner();
                let note_no = inner_rules.next().unwrap().as_str().parse::<isize>()?;

                let mut params = [None; 4];

                for (i, pair) in inner_rules.enumerate() {
                    if let Ok(num) = extract_number(pair) {
                        params[i] = Some(num);
                    }
                }

                Ok(MmlAst::NNote {
                    note_no,
                    length: params[0],
                    gate: params[1],
                    velocity: params[2],
                    timing: params[3],
                })
            }
            Rule::rest => Ok(MmlAst::Rest(None)),
            Rule::length => {
                let num = extract_number(pair)?;
                Ok(MmlAst::Length(num))
            }
            Rule::octave => {
                let num = extract_number(pair)?;
                Ok(MmlAst::Octave(num))
            }
            Rule::pitch_bend => {
                let num = extract_number(pair)?;
                Ok(MmlAst::PitchBend(num))
            }
            Rule::gate => {
                let num = extract_number(pair)?;
                Ok(MmlAst::Gate(num))
            }
            Rule::velocity => {
                let (value, random) = extract_value_and_random(pair)?;
                Ok(MmlAst::Velocity { value, random })
            }
            Rule::timing => {
                let (value, random) = extract_value_and_random(pair)?;
                Ok(MmlAst::Timing { value, random })
            }
            Rule::control_change => {
                let (controller, value, on_time) = extract_control_change(pair)?;
                Ok(MmlAst::ControlChange {
                    controller,
                    value,
                    on_time,
                })
            }
            Rule::macro_def => {
                let content = pair.as_str().to_string();
                Ok(MmlAst::Macro(content))
            }
            Rule::voice_select => {
                let (number, bank_lsb, bank_msb) = extract_voice_select(pair)?;
                Ok(MmlAst::VoiceSelect {
                    number,
                    bank_lsb,
                    bank_msb,
                })
            }
            Rule::octave_up => Ok(MmlAst::OctaveUp),
            Rule::octave_down => Ok(MmlAst::OctaveDown),
            Rule::velocity_up => {
                let num = extract_optional_number(pair)?;
                Ok(MmlAst::VelocityUp(num))
            }
            Rule::velocity_down => {
                let num = extract_optional_number(pair)?;
                Ok(MmlAst::VelocityDown(num))
            }
            Rule::range_comment => {
                let content = extract_comment_content(pair);
                Ok(MmlAst::Comment {
                    kind: CommentKind::RangeComment,
                    content,
                })
            }
            Rule::line_comment_debug => {
                let content = extract_comment_content(pair);
                Ok(MmlAst::Comment {
                    kind: CommentKind::LineCommentDebug,
                    content,
                })
            }
            Rule::line_comment => {
                let content = extract_comment_content(pair);
                Ok(MmlAst::Comment {
                    kind: CommentKind::LineComment,
                    content,
                })
            }
            Rule::loop_begin => {
                let num = extract_optional_number(pair)?;
                Ok(MmlAst::LoopBegin(num))
            }
            Rule::loop_break => Ok(MmlAst::LoopBreak),
            Rule::loop_end => Ok(MmlAst::LoopEnd),
            Rule::harmony => {
                let (notes, length, gate) = extract_harmony(pair)?;
                Ok(MmlAst::Harmony {
                    notes,
                    length,
                    gate,
                })
            }
            Rule::rhythm_macro_define => {
                let (name, definition) = extract_rhythm_macro_define(pair)?;
                Ok(MmlAst::RhythmMacroDefine {
                    name,
                    definition: Box::new(definition),
                })
            }
            Rule::group_notes => {
                let (notes, length) = extract_group_notes(pair)?;
                Ok(MmlAst::GroupedNotes { notes, length })
            }
            Rule::octave_up_once => Ok(MmlAst::OctaveUpOnce),
            Rule::octave_down_once => Ok(MmlAst::OctaveDownOnce),
            Rule::play_from_here => Ok(MmlAst::PlayFromHere),
            Rule::tie_slur => Ok(MmlAst::TieSlur),
            _ => Err(anyhow::anyhow!("Unknown rule: {:?}", pair)),
        };

        match ast {
            Ok(ast) => Ok(Some(ast)),
            Err(e) => {
                eprintln!("Error parsing command: {:?}", e);
                Ok(None)
            }
        }
    }
}

// 以下、ヘルパー関数の実装

fn extract_number(pair: Pair<Rule>) -> Result<isize> {
    let num_pair = pair
        .into_inner()
        .next()
        .ok_or_else(|| anyhow::anyhow!("Expected number"))?;
    let num = num_pair.as_str().parse::<isize>()?;
    Ok(num)
}

fn extract_optional_number(pair: Pair<Rule>) -> Result<Option<isize>> {
    let mut inner_rules = pair.into_inner();
    if let Some(num_pair) = inner_rules.next() {
        let num = num_pair.as_str().parse::<isize>()?;
        Ok(Some(num))
    } else {
        Ok(None)
    }
}

fn extract_value_and_random(pair: Pair<Rule>) -> Result<(isize, Option<isize>)> {
    let mut inner_rules = pair.into_inner();
    let value_pair = inner_rules
        .next()
        .ok_or_else(|| anyhow::anyhow!("Expected value"))?;
    let value = value_pair.as_str().parse::<isize>()?;
    let random = if let Some(random_pair) = inner_rules.next() {
        Some(random_pair.as_str().parse::<isize>()?)
    } else {
        None
    };
    Ok((value, random))
}

fn extract_control_change(
    pair: Pair<Rule>,
) -> Result<(isize, isize, Option<(isize, isize, isize)>)> {
    let mut inner_rules = pair.into_inner();
    let controller_pair = inner_rules
        .next()
        .ok_or_else(|| anyhow::anyhow!("Expected controller number"))?;
    let controller = controller_pair.as_str().parse::<isize>()?;
    let value_pair = inner_rules
        .next()
        .ok_or_else(|| anyhow::anyhow!("Expected value"))?;
    let value = value_pair.as_str().parse::<isize>()?;
    let on_time = if let Some(on_time_pair) = inner_rules.next() {
        let mut time_values = on_time_pair.into_inner();
        let low = time_values
            .next()
            .ok_or_else(|| anyhow::anyhow!("Expected low value"))?
            .as_str()
            .parse::<isize>()?;
        let high = time_values
            .next()
            .ok_or_else(|| anyhow::anyhow!("Expected high value"))?
            .as_str()
            .parse::<isize>()?;
        let len = time_values
            .next()
            .ok_or_else(|| anyhow::anyhow!("Expected length value"))?
            .as_str()
            .parse::<isize>()?;
        Some((low, high, len))
    } else {
        None
    };
    Ok((controller, value, on_time))
}

fn extract_voice_select(pair: Pair<Rule>) -> Result<(isize, Option<isize>, Option<isize>)> {
    let mut inner_rules = pair.into_inner();
    let number_pair = inner_rules
        .next()
        .ok_or_else(|| anyhow::anyhow!("Expected voice number"))?;
    let number = number_pair.as_str().parse::<isize>()?;
    let bank_lsb = if let Some(bank_lsb_pair) = inner_rules.next() {
        Some(bank_lsb_pair.as_str().parse::<isize>()?)
    } else {
        None
    };
    let bank_msb = if let Some(bank_msb_pair) = inner_rules.next() {
        Some(bank_msb_pair.as_str().parse::<isize>()?)
    } else {
        None
    };
    Ok((number, bank_lsb, bank_msb))
}

fn extract_comment_content(pair: Pair<Rule>) -> String {
    pair.into_inner().as_str().to_string()
}

fn extract_harmony(pair: Pair<Rule>) -> Result<(Vec<char>, Option<isize>, Option<isize>)> {
    let mut inner_rules = pair.into_inner();
    let notes_str = inner_rules
        .next()
        .ok_or_else(|| anyhow::anyhow!("Expected notes"))?
        .as_str();
    let notes = notes_str.chars().collect();
    let mut length = None;
    let mut gate = None;
    if let Some(param_pair) = inner_rules.next() {
        length = Some(param_pair.as_str().parse::<isize>()?);
        if let Some(gate_pair) = inner_rules.next() {
            gate = Some(gate_pair.as_str().parse::<isize>()?);
        }
    }
    Ok((notes, length, gate))
}

fn extract_rhythm_macro_define(pair: Pair<Rule>) -> Result<(char, MmlAst)> {
    let mut inner_rules = pair.into_inner();
    let name_pair = inner_rules
        .next()
        .ok_or_else(|| anyhow::anyhow!("Expected macro name"))?;
    let name = name_pair.as_str().chars().next().unwrap();
    let definition_pair = inner_rules
        .next()
        .ok_or_else(|| anyhow::anyhow!("Expected macro definition"))?;
    let definition = MmlAst::parse_command(definition_pair)?;

    if let Some(definition) = definition {
        Ok((name, definition))
    } else {
        Err(anyhow::anyhow!("Failed to parse macro definition"))
    }
}

fn extract_group_notes(pair: Pair<Rule>) -> Result<(Vec<MmlAst>, Option<isize>)> {
    let mut inner_rules = pair.into_inner();
    let mut notes: Vec<MmlAst> = Vec::new();
    for note_pair in inner_rules.by_ref() {
        let note = MmlAst::parse_command(note_pair)?;
        if let Some(note) = note {
            notes.push(note);
        }
    }
    let length = if let Some(length_pair) = inner_rules.next() {
        Some(length_pair.as_str().parse::<isize>()?)
    } else {
        None
    };

    Ok((notes, length))
}
