/*

TODO
- handle errors.
- parity

*/

use std::{thread,time};

use gpio_cdev::{Chip, LineRequestFlags, Line, EventType};

const LOW:  u8= 0;
const HIGH: u8 = 1;

#[derive(Debug)]
struct Event {
	timestamp: time::Instant,
	event_type: EventType,
}

impl Event {
	pub fn new(timestamp: time::Instant, event_type: EventType) -> Self {
		Event {timestamp, event_type}
	}
}

fn data_to_values(data: &Vec<u8>) -> Vec<u32>{

	data
		.chunks(4)
		.map(|chunk| {
			// is this a fold?
			let mut total : u32 = 0;
			for (i,bit) in chunk.iter().enumerate() {
				let exp = (3-i) as u8;
				total += *bit as u32 * (2 as u32).pow(exp as u32) as u32;
			}
			total
		})
		.collect::<Vec<u32>>()
		.chunks(4)
		.map(|chunk| {
			// is this a fold?
			let mut total : u32 = 0;
			match chunk.len() {
				4 => {
					for (i,digit) in chunk.iter().enumerate() {
						let exp = (3-i) as u32;
						total += *digit * (16 as u32).pow(exp);
					}
					total
				},
				2 => {
					for (i,digit) in chunk.iter().enumerate() {
						let exp = (1-i) as u32;
						total += *digit * (16 as u32).pow(exp);
					}
					total
				},
				_ => 0, // ERROR
			}
		})
		.collect()
}

fn events_to_data(events: &[Event]) -> Vec<u8> {
	events[2..]
		.windows(2)
		.map(|pair| {
			let prev = pair.get(0).unwrap();
			let next = pair.get(1).unwrap();
			match next.event_type {
				EventType::FallingEdge => Some(next.timestamp - prev.timestamp),
				EventType::RisingEdge => None,
			}
		})
		.filter(|&d| d.is_some())
		.map(|elapsed| {
			if elapsed.unwrap().as_micros() > 35 {1} else {0}
		}).collect()
}

fn get_line() -> Line {
	let mut chip = Chip::new("/dev/gpiochip0").unwrap();
	chip.get_line(4).unwrap()
}

fn read_bits(line: &Line, events: &mut Vec<Event>) {
	let input = line.request(
		LineRequestFlags::INPUT,
		HIGH,
		"read-data").unwrap();

	let mut last_state = input.get_value().unwrap();

	// make sure this loop doesn't hang
	loop {
		let new_state = input.get_value().unwrap();
		if new_state != last_state {
			let timestamp = time::Instant::now();
			let event_type = if last_state == LOW && new_state == HIGH {
				EventType::RisingEdge
			} else {
				EventType::FallingEdge
			};
			events.push(Event::new(timestamp, event_type));
			if events.len() >= 83 {
				break;
			}
			last_state = new_state;
		}
	}
}

fn prime_read(line: &Line) {
	let output = line.request(
		LineRequestFlags::OUTPUT,
		HIGH,
		"request-data").unwrap();
		output.set_value(0).unwrap();
		thread::sleep(time::Duration::from_millis(3));
}

fn main() {

	let line = get_line();

	loop {
		prime_read(&line);
		let mut events : Vec<Event> = Vec::with_capacity(83);
		read_bits(&line, &mut events);
		let data = events_to_data(&events);
		let values = data_to_values(&data);
		println!("Temperature: {}, Humidity: {}", values[1] as f32/10., values[0] as f32/10.);
		thread::sleep(time::Duration::from_secs(10));
	}
}
