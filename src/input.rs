use super::*;

#[derive(Debug, PartialEq)]
pub struct Keyboard {
    handlers: Vec<String>,
    name: String,
}
impl Keyboard {
    // Parses a Keyboard object from a block read from INPUT_DEVICE_LIST file
    //
    // Example block:
    // I: Bus=0003 Vendor=046d Product=c33a Version=0111
    // N: Name="Logitech G413 Carbon Mechanical Gaming Keyboard"
    // P: Phys=usb-0000:0b:00.3-4/input0
    // S:
    // Sysfs=/devices/pci0000:00/0000:00:08.1/0000:0b:00.3/usb3/3-4/3-4:1.0/0003:046D:C33A.0001/input/input2
    // U: Uniq=188338553234
    // H: Handlers=sysrq kbd event2 leds
    // B: PROP=0
    // B: EV=120013
    // B: KEY=1000000000007 ff9f207ac14057ff febeffdfffefffff fffffffffffffffe
    // B: MSC=10
    // B: LED=7
    //
    // In this example the file with all interesting events will be located in:
    //   /dev/input/event2
    pub fn new(inp: &str) -> Keyboard {
        info!("Parsing keyboard object");
        trace!("From input:\n{}", &inp);
        let lines = inp.split('\n');
        let mut handlers = Vec::new();
        let mut name = String::new();

        for line in lines {
            if line.starts_with(NAME_PREFIX) {
                name = line[NAME_PREFIX.len()..line.len() - 1].to_string();
            } else if line.starts_with(HANDLER_PREFIX) {
                handlers = line[HANDLER_PREFIX.len()..]
                    .split(' ')
                    // Only interested in event handlers
                    .filter(|h| h.starts_with("event"))
                    .map(|h| h.to_string())
                    .collect();
            }
        }
        info!("Found keyboard (name={}, handlers={:?})", name, &handlers);
        Keyboard { handlers, name }
    }
    // Attempts to open all event handler files
    pub fn handlers(&self) -> Vec<Result<File, std::io::Error>> {
        info!("Getting event file handles");
        self.handlers
            .iter()
            .map(|h| {
                let mut path = PathBuf::from(DEV_INP);
                path.push(h);
                trace!("Opening {}", path.as_path().display());
                File::open(path)
            })
            .collect()
    }
}

pub fn read_input_devices() -> Result<Vec<Keyboard>, std::io::Error> {
    info!("Reading device list from {}", INPUT_DEVICE_LIST);
    let device_list = fs::read_to_string(INPUT_DEVICE_LIST)?;

    // All devices with EV=120013
    Ok(device_list
        .split("\n\n")
        .filter(|dev| dev.contains(KEYBOARD_INPUT_ID))
        .map(|k| Keyboard::new(&k))
        .collect())
}

#[derive(Debug)]
#[repr(C)]
pub struct InputEvent {
    _tv_sec: isize,  // from timeval struct
    _tv_usec: isize, // from timeval struct
    pub type_: u16,
    pub code: u16,
    pub value: i32,
}
impl InputEvent {
    pub fn new(buf: &[u8]) -> Result<InputEvent, std::io::Error> {
        let mut rdr = Cursor::new(&buf);
        Ok(InputEvent {
            _tv_sec: rdr
                .read_int::<LittleEndian>(SIZE_OF_ISIZE)?
                .try_into()
                .unwrap(),
            _tv_usec: rdr
                .read_int::<LittleEndian>(SIZE_OF_ISIZE)?
                .try_into()
                .unwrap(),
            type_: rdr.read_u16::<LittleEndian>()?,
            code: rdr.read_u16::<LittleEndian>()?,
            value: rdr.read_i32::<LittleEndian>()?,
        })
    }
    pub fn is_shift(&self) -> bool {
        match self.as_enum() {
            Key::LShift | Key::RShift => true,
            _ => false,
        }
    }
    pub fn is_ctrl(&self) -> bool {
        match self.as_enum() {
            Key::LCtrl | Key::RCtrl => true,
            _ => false,
        }
    }
    pub fn is_alt(&self) -> bool {
        match self.as_enum() {
            Key::LAlt | Key::RAlt => true,
            _ => false,
        }
    }
    pub fn is_super(&self) -> bool {
        match self.as_enum() {
            Key::Super => true,
            _ => false,
        }
    }
    pub fn is_key_event(&self) -> bool {
        self.type_ == KEY_EV
    }
    pub fn is_key_press(&self) -> bool {
        self.value == KEY_PRESS
    }
    pub fn is_key_release(&self) -> bool {
        self.value == KEY_RELEASE
    }
    pub fn as_enum(&self) -> Key {
        Key::from_code(self.code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_keyboard() {
        let kb_txt = "I: Bus=0003 Vendor=046d Product=c33a Version=0111
N: Name=\"Logitech G413 Carbon Mechanical Gaming Keyboard\"
P: Phys=usb-0000:0b:00.3-4/input0
S:
Sysfs=/devices/pci0000:00/0000:00:08.1/0000:0b:00.3/usb3/3-4/3-4:1.0/0003:046D:C33A.0001/input/input2
U: Uniq=188338553234
H: Handlers=sysrq kbd event2 leds
B: PROP=0
B: EV=120013
B: KEY=1000000000007 ff9f207ac14057ff febeffdfffefffff fffffffffffffffe
B: MSC=10
B: LED=7";
        let kb = Keyboard {
            handlers: vec!["event2".to_string()],
            name: "Logitech G413 Carbon Mechanical Gaming Keyboard".to_string(),
        };
        let parsed_kb = Keyboard::new(&kb_txt);
        assert_eq!(kb, parsed_kb);
    }
}
