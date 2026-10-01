use protocol::scpi::*;
use protocol::{Edge, Led, Mode, Position, Rgb};

const AMBER: Rgb = Rgb { r: 0xFF, g: 0x8C, b: 0x00 };
const GREEN: Rgb = Rgb { r: 0x00, g: 0xFF, b: 0x00 };
const BLUE: Rgb = Rgb { r: 0x00, g: 0x00, b: 0xFF };

fn led(rgb: Rgb, mode: Mode) -> Led {
    Led { rgb, mode }
}

fn text<T>(write: impl Fn(&mut String) -> T) -> String {
    let mut s = String::new();
    write(&mut s);
    s
}

fn every_command() -> Vec<Command> {
    let mut all = vec![
        Command::Identify,
        Command::Protocol,
        Command::NextError,
        Command::NextKey,
        Command::SetAll([led(AMBER, Mode::Breathe), led(GREEN, Mode::Solid), led(BLUE, Mode::Blink)]),
    ];
    for pos in [Position::Left, Position::Middle, Position::Right] {
        for mode in [Mode::Off, Mode::Solid, Mode::Breathe, Mode::Blink, Mode::Wave] {
            all.push(Command::Set(pos, led(AMBER, mode)));
        }
        all.push(Command::Flash(pos, GREEN));
    }
    all
}

mod commands {
    use super::*;

    #[test]
    fn every_command_round_trips_through_its_text() {
        for cmd in every_command() {
            let line = text(|s| write_command(&cmd, s));
            assert_eq!(parse_command(&line), Ok(cmd), "{line:?}");
        }
    }

    #[test]
    fn written_commands_are_the_documented_text() {
        let cases = [
            (Command::Identify, "*IDN?"),
            (Command::Protocol, "SYST:PROT?"),
            (Command::NextError, "SYST:ERR?"),
            (Command::NextKey, "KEY:EVEN?"),
            (
                Command::SetAll([led(AMBER, Mode::Breathe), led(GREEN, Mode::Solid), led(BLUE, Mode::Blink)]),
                "LED:ALL #FF8C00,BREATHE,#00FF00,SOLID,#0000FF,BLINK",
            ),
            (Command::Set(Position::Middle, led(GREEN, Mode::Off)), "LED2 #00FF00,OFF"),
            (Command::Flash(Position::Right, BLUE), "LED3:FLAS #0000FF"),
        ];
        for (cmd, expected) in cases {
            assert_eq!(text(|s| write_command(&cmd, s)), expected);
        }
    }

    #[test]
    fn headers_accept_long_and_short_forms_in_any_case() {
        for line in ["SYST:PROT?", "SYSTEM:PROTOCOL?", "system:protocol?", "Syst:Protocol?", ":SYST:PROT?"] {
            assert_eq!(parse_command(line), Ok(Command::Protocol), "{line:?}");
        }
        for line in ["KEY:EVEN?", "key:event?"] {
            assert_eq!(parse_command(line), Ok(Command::NextKey), "{line:?}");
        }
        for line in ["LED1:FLAS #00FF00", "led1:flash #00ff00"] {
            assert_eq!(parse_command(line), Ok(Command::Flash(Position::Left, GREEN)), "{line:?}");
        }
    }

    #[test]
    fn modes_accept_long_and_short_forms() {
        for (word, mode) in [("SOL", Mode::Solid), ("solid", Mode::Solid), ("BRE", Mode::Breathe), ("blin", Mode::Blink), ("off", Mode::Off), ("WAV", Mode::Wave), ("wave", Mode::Wave)] {
            let line = format!("LED1 #FF8C00,{word}");
            assert_eq!(parse_command(&line), Ok(Command::Set(Position::Left, led(AMBER, mode))), "{line:?}");
        }
    }

    #[test]
    fn surrounding_whitespace_and_spaces_after_commas_are_ignored() {
        assert_eq!(
            parse_command("  LED3   #0000FF , BLINK \r"),
            Ok(Command::Set(Position::Right, led(BLUE, Mode::Blink)))
        );
    }

    #[test]
    fn an_unknown_header_is_undefined() {
        for line in ["FOO?", "SYST:FOO?", "KEY:EVEN", "SYST:PROT", "*IDN", "LED1?", "LED:ALL?", "LED1:FOO #000000", "SYSTE:PROT?", "SYSTEMS:PROT?"] {
            assert_eq!(parse_command(line), Err(ErrorCode::UndefinedHeader), "{line:?}");
        }
    }

    #[test]
    fn an_led_number_colour_or_mode_out_of_range_is_data_out_of_range() {
        for line in ["LED0 #000000,OFF", "LED4 #000000,OFF", "LED1 #00000,OFF", "LED1 000000,OFF", "LED1 #GG0000,OFF", "LED1 #000000,DIM", "LED4:FLAS #000000"] {
            assert_eq!(parse_command(line), Err(ErrorCode::DataOutOfRange), "{line:?}");
        }
    }

    #[test]
    fn a_wrong_parameter_count_is_a_command_error() {
        for line in ["LED1 #000000", "LED1 #000000,OFF,OFF", "LED:ALL #000000,OFF", "LED1:FLAS", "*IDN? 1", "KEY:EVEN? now", ""] {
            assert_eq!(parse_command(line), Err(ErrorCode::CommandError), "{line:?}");
        }
    }

    #[test]
    fn only_queries_expect_a_reply() {
        for cmd in every_command() {
            let is_query = text(|s| write_command(&cmd, s)).contains('?');
            assert_eq!(cmd.is_query(), is_query, "{cmd:?}");
        }
    }
}

mod replies {
    use super::*;

    fn round_trip(query: &Command, reply: Reply) {
        let line = text(|s| write_reply(&reply, s));
        assert_eq!(parse_reply(query, &line), Ok(reply), "{line:?}");
    }

    #[test]
    fn identity_names_maker_model_serial_and_version() {
        let reply = Reply::Identity { serial: "TRITON-0123456789ABCDEF", version: "0.1.0" };
        assert_eq!(
            text(|s| write_reply(&reply, s)),
            "ShortArrow,herdr-triton,TRITON-0123456789ABCDEF,0.1.0"
        );
        round_trip(&Command::Identify, reply);
    }

    #[test]
    fn protocol_is_a_bare_number() {
        assert_eq!(text(|s| write_reply(&Reply::Protocol(PROTOCOL_VERSION), s)), "3");
        round_trip(&Command::Protocol, Reply::Protocol(PROTOCOL_VERSION));
    }

    #[test]
    fn errors_carry_their_scpi_code_and_message() {
        let cases = [
            (None, "0,\"No error\""),
            (Some(ErrorCode::CommandError), "-100,\"Command error\""),
            (Some(ErrorCode::UndefinedHeader), "-113,\"Undefined header\""),
            (Some(ErrorCode::DataOutOfRange), "-222,\"Data out of range\""),
            (Some(ErrorCode::QueueOverflow), "-350,\"Queue overflow\""),
        ];
        for (code, expected) in cases {
            assert_eq!(text(|s| write_reply(&Reply::Error(code), s)), expected);
            round_trip(&Command::NextError, Reply::Error(code));
        }
    }

    #[test]
    fn key_events_name_position_and_edge_or_none() {
        assert_eq!(text(|s| write_reply(&Reply::Key(None), s)), "NONE");
        assert_eq!(
            text(|s| write_reply(&Reply::Key(Some((Position::Middle, Edge::Down))), s)),
            "MIDDLE,DOWN"
        );
        round_trip(&Command::NextKey, Reply::Key(None));
        for pos in [Position::Left, Position::Middle, Position::Right] {
            for edge in [Edge::Down, Edge::Up] {
                round_trip(&Command::NextKey, Reply::Key(Some((pos, edge))));
            }
        }
    }

    #[test]
    fn a_reply_that_does_not_fit_its_query_is_rejected() {
        assert_eq!(parse_reply(&Command::NextKey, "SIDEWAYS,DOWN"), Err(ReplyError));
        assert_eq!(parse_reply(&Command::Protocol, "two"), Err(ReplyError));
        assert_eq!(parse_reply(&Command::Identify, "ShortArrow,herdr-triton"), Err(ReplyError));
        assert_eq!(
            parse_reply(&Command::Identify, "Keysight Technologies,34461A,MY12345678,A.03.01"),
            Err(ReplyError)
        );
        assert_eq!(parse_reply(&Command::SetAll([led(AMBER, Mode::Off); 3]), "1"), Err(ReplyError));
    }
}

mod lines {
    use super::*;

    fn lines_of(bytes: &[u8]) -> Vec<Result<String, ErrorCode>> {
        let mut buf = LineBuffer::new();
        bytes
            .iter()
            .filter_map(|b| buf.push(*b).map(|r| r.map(str::to_owned)))
            .collect()
    }

    #[test]
    fn a_line_ends_at_newline_and_drops_a_carriage_return() {
        assert_eq!(lines_of(b"*IDN?\r\nKEY:EVEN?\n"), vec![Ok("*IDN?".into()), Ok("KEY:EVEN?".into())]);
    }

    #[test]
    fn a_line_of_max_length_is_kept() {
        let line = "A".repeat(MAX_LINE);
        assert_eq!(lines_of(format!("{line}\n").as_bytes()), vec![Ok(line)]);
    }

    #[test]
    fn a_longer_line_is_a_command_error_and_the_next_line_still_reads() {
        let long = "A".repeat(MAX_LINE + 1);
        assert_eq!(
            lines_of(format!("{long}\n*IDN?\n").as_bytes()),
            vec![Err(ErrorCode::CommandError), Ok("*IDN?".into())]
        );
    }

    #[test]
    fn non_utf8_is_a_command_error() {
        assert_eq!(lines_of(b"\xff\xfe\n"), vec![Err(ErrorCode::CommandError)]);
    }
}
