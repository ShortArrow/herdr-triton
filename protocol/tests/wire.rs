use protocol::*;

const WHITE: Rgb = Rgb { r: 255, g: 255, b: 255 };

fn decode_all<M: Message>(bytes: &[u8]) -> Vec<Result<M, DecodeError>> {
    let mut decoder = Decoder::<M>::new();
    bytes.iter().filter_map(|b| decoder.push(*b)).collect()
}

fn frame_of<M: Message>(msg: &M) -> Vec<u8> {
    let mut buf = [0u8; MAX_FRAME_LEN];
    encode(msg, &mut buf).to_vec()
}

fn every_device_message() -> Vec<DeviceMessage> {
    let mut all = vec![
        DeviceMessage::Ready { protocol: 0 },
        DeviceMessage::Ready { protocol: u16::MAX },
    ];
    for pos in [Position::Left, Position::Middle, Position::Right] {
        for edge in [Edge::Down, Edge::Up] {
            all.push(DeviceMessage::Key { pos, edge });
        }
    }
    all
}

fn every_host_message_shape() -> Vec<HostMessage> {
    let mut all = Vec::new();
    for mode in [Mode::Off, Mode::Solid, Mode::Breathe, Mode::Blink] {
        let led = Led { rgb: WHITE, mode };
        all.push(HostMessage::Frame([led; 3]));
    }
    for pos in [Position::Left, Position::Middle, Position::Right] {
        all.push(HostMessage::Flash { pos, rgb: WHITE });
    }
    all
}

#[test]
fn every_device_message_round_trips() {
    for msg in every_device_message() {
        assert_eq!(decode_all::<DeviceMessage>(&frame_of(&msg)), vec![Ok(msg)]);
    }
}

#[test]
fn every_host_message_shape_round_trips() {
    for msg in every_host_message_shape() {
        assert_eq!(decode_all::<HostMessage>(&frame_of(&msg)), vec![Ok(msg)]);
    }
}

#[test]
fn a_frame_contains_zero_only_as_its_last_byte() {
    let msg = HostMessage::Frame([Led { rgb: Rgb { r: 0, g: 0, b: 0 }, mode: Mode::Off }; 3]);
    let frame = frame_of(&msg);
    assert_eq!(frame.last(), Some(&0));
    assert!(!frame[..frame.len() - 1].contains(&0));
}

#[test]
fn the_largest_messages_fit_in_max_frame_len() {
    let widest = HostMessage::Frame([Led { rgb: WHITE, mode: Mode::Blink }; 3]);
    assert!(frame_of(&widest).len() <= MAX_FRAME_LEN);
    let ready = DeviceMessage::Ready { protocol: u16::MAX };
    assert!(frame_of(&ready).len() <= MAX_FRAME_LEN);
}

#[test]
fn a_frame_split_across_pushes_decodes_once_at_its_terminator() {
    let msg = DeviceMessage::Key { pos: Position::Middle, edge: Edge::Down };
    let frame = frame_of(&msg);
    let mut decoder = Decoder::<DeviceMessage>::new();
    for b in &frame[..frame.len() - 1] {
        assert_eq!(decoder.push(*b), None);
    }
    assert_eq!(decoder.push(0), Some(Ok(msg)));
}

#[test]
fn back_to_back_frames_decode_in_order() {
    let first = DeviceMessage::Key { pos: Position::Left, edge: Edge::Down };
    let second = DeviceMessage::Key { pos: Position::Left, edge: Edge::Up };
    let stream = [frame_of(&first), frame_of(&second)].concat();
    assert_eq!(decode_all::<DeviceMessage>(&stream), vec![Ok(first), Ok(second)]);
}

#[test]
fn garbage_is_reported_and_the_next_frame_still_decodes() {
    let msg = DeviceMessage::Key { pos: Position::Right, edge: Edge::Up };
    let stream = [vec![0x05, 0xff, 0xff, 0x00], frame_of(&msg)].concat();
    assert_eq!(
        decode_all::<DeviceMessage>(&stream),
        vec![Err(DecodeError::Malformed), Ok(msg)]
    );
}

#[test]
fn an_overlong_frame_is_reported_once_and_the_next_frame_still_decodes() {
    let msg = DeviceMessage::Ready { protocol: PROTOCOL_VERSION };
    let stream = [vec![0x01; MAX_FRAME_LEN * 3], vec![0x00], frame_of(&msg)].concat();
    assert_eq!(
        decode_all::<DeviceMessage>(&stream),
        vec![Err(DecodeError::Overflow), Ok(msg)]
    );
}

#[test]
fn a_stray_terminator_yields_nothing() {
    assert_eq!(decode_all::<DeviceMessage>(&[0x00, 0x00]), vec![]);
}

#[test]
fn a_host_frame_does_not_decode_as_a_device_message() {
    let frame = frame_of(&HostMessage::Frame([Led { rgb: WHITE, mode: Mode::Solid }; 3]));
    assert_eq!(decode_all::<DeviceMessage>(&frame), vec![Err(DecodeError::Malformed)]);
}

#[test]
fn a_valid_message_followed_by_extra_bytes_in_one_frame_is_malformed() {
    // postcard of Key { Left, Down } is [1, 0, 0]; one extra byte 7 follows.
    // COBS of [1, 0, 0, 7] is [2, 1, 1, 2, 7], then the terminator.
    let stream = [0x02, 0x01, 0x01, 0x02, 0x07, 0x00];
    assert_eq!(decode_all::<DeviceMessage>(&stream), vec![Err(DecodeError::Malformed)]);
}

#[test]
fn overflow_starts_one_byte_past_max_frame_len() {
    let at_limit = [vec![0x01; MAX_FRAME_LEN - 1], vec![0x00]].concat();
    assert_eq!(decode_all::<DeviceMessage>(&at_limit), vec![Err(DecodeError::Malformed)]);
    let past_limit = [vec![0x01; MAX_FRAME_LEN], vec![0x00]].concat();
    assert_eq!(decode_all::<DeviceMessage>(&past_limit), vec![Err(DecodeError::Overflow)]);
}
