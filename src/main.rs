use std::io;
use std::io::Error;
use std::cell::Cell;
use std::convert::TryInto;
use std::collections::HashSet;

const KEYEVENT_KEYUP: u32 = 0x002;

#[link(name="User32")]
extern "system"{
    fn FindWindowW(lpClassName: *mut u16, lpWindowName: *mut u16) -> usize;
    fn PostMessageW(hWnd: usize, msg: u32, wParam: usize, lParam: usize) -> usize;
    fn GetForegroundWindow() -> usize;
    fn SendInput(cInputs: u32, pInputs: *mut Input, cbSize: i32) -> u32;
}
///Different types of input for the 'typ' field on 'Input'
#[repr(C)]
#[derive(Clone, Copy)]
enum InputType{
    Mouse    = 0,
    Keyboard = 1,
    Hardware = 2,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct Input{
    typ: InputType,
    union: InputUnion,
}

#[repr(C)]
#[derive(Clone, Copy)]
union InputUnion{
    mouse:      MouseInput,
    keyboard:   KeyboardInput,
    hardware:   HardwareInput,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct KeyboardInput{
    vk:         u16,
    scan_code:  u16,
    flags:      u32,
    time:       u32,
    extra_info: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct MouseInput{
    dx:         i32,
    dy:         i32,
    mouse_data: u32,
    flags:      u32,
    time:       u32,
    extra_info: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct HardwareInput{
    msg:    u32,
    lparam: u16,
    hparam: u16,
}


/// Convert a Rust UTF-8 'string' into a NULL-terminated UTF-16 vector
fn str_to_utf16(string: &str) -> Vec<u16> {
    let mut ret: Vec<u16> = string.encode_utf16().collect();
    ret.push(0);
    ret
}
/// Different types of message for 'SendMessage()'
#[repr(u32)]
enum MsgType{
    KeyDown         = 0x100,
    KeyUp           = 0x101,
}

#[repr(u8)]
enum KeyCode {
    Back    = 0x08,
    Tab     = 0x09,
    Return  = 0x10,
    Control = 0x11,
    Alt     = 0x12,
    Left    = 0x25,
    Up      = 0x26,
    Right   = 0x27,
    Down    = 0x28,
}

/// An active handle to a window
struct Window{
    /// Handle to window which we have opened
    hwnd: usize,

    /// Seed for an RNG
    seed: Cell<u64>,

    /// Keys which seem interesting
    interesting_keys: Vec<u8>,
}
impl Window{
    /// Find a window with 'title', and return a new 'Window' object
    fn attach(title: &str) -> io::Result<Self>{
        /// convert the title to UTF-16
        let mut title = str_to_utf16(title);

        ///Finds the window with 'title'
        let ret = unsafe {
            FindWindowW(std::ptr::null_mut(), title.as_mut_ptr())
        };
        /// Generate some interesting keys
        let mut interesting_keys = Vec::new();
        interesting_keys.push(KeyCode::Left as u8);
        interesting_keys.push(KeyCode::Up as u8);
        interesting_keys.push(KeyCode::Down as u8);
        interesting_keys.push(KeyCode::Right as u8);
        interesting_keys.push(KeyCode::Tab as u8);
        interesting_keys.push(KeyCode::Right as u8);
        interesting_keys.extend_from_slice(
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789()-+=/*@#"
            );



        if ret !=0 {
            /// Sucessfully got a handle to the window
            return Ok(Window {
                hwnd: ret,
                seed: Cell::new(unsafe {core::arch::x86_64::_rdtsc()}),
                interesting_keys,

            });
        }
        else{
            /// FindWindow() failed, return out the corresponding error
            Err(Error::last_os_error())
        }
    }

    /// Get a random 64-bit number using xorshift
    fn rand(&self) -> usize {
        let mut seed = self.seed.get();
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 43;
        self.seed.set(seed);
        seed as usize

    }


    fn keystream(&self, inputs: &[KeyboardInput]) -> io::Result<()>{
     /// Generate an array to pass directly to 'SendInput()'
            let mut win_inputs = Vec::new();

            ///Create inputs based on each keyboard input
            for &input in inputs.iter(){
            win_inputs.push(Input {
                typ: InputType::Keyboard,
                union: InputUnion {
                    keyboard: input

                }
            });
            }

            let res = unsafe {
                SendInput(
                    win_inputs.len().try_into().unwrap(),
                    win_inputs.as_mut_ptr(),
                    std::mem::size_of::<Input>().try_into().unwrap())
        };
            if (res as usize) != inputs.len(){
                Err(Error::last_os_error())
            }
            else{
                Ok(())
            }
    }

    fn press(&self, key: u16) -> io::Result<()> {
        self.keystream(&[
        KeyboardInput{
                vk: key,
                scan_code: 0,
                flags: 0,
                time: 0,
                extra_info: 0,
            },
            KeyboardInput{
                vk: key,
                scan_code: 0,
                flags: KEYEVENT_KEYUP,
                time: 0,
                extra_info: 0,
            },
        ])
    }

    fn alt_press(&self, key: u16) -> io::Result<()> {
	if key == KeyCode::Tab as u16 || key == b' '  as u16 || key == b'H' as u16{
	   return Ok(());
	}
        self.keystream(&[
            KeyboardInput{
                vk: KeyCode::Alt as u16,
                scan_code: 0,
                flags: 0,
                time: 0,
                extra_info: 0,
            },

            KeyboardInput{
                vk: key,
                scan_code: 0,
                flags: 0,
                time: 0,
                extra_info: 0,
            },

            KeyboardInput{
                vk: key,
                scan_code: 0,
                flags: KEYEVENT_KEYUP,
                time: 0,
                extra_info: 0,
            },

            KeyboardInput{
                vk: KeyCode::Alt as u16,
                scan_code: 0,
                flags: KEYEVENT_KEYUP,
                time: 0,
                extra_info: 0,
            }
        ])
    }

    fn ctrl_press(&self, key: u16) -> io::Result<()> {
        self.keystream(&[
             KeyboardInput{
                vk: KeyCode::Control as u16,
                scan_code: 0,
                flags: 0,
                time: 0,
                extra_info: 0,
            },
            KeyboardInput{
                vk: key,
                scan_code: 0,
                flags: 0,
                time: 0,
                extra_info: 0,
            },
            KeyboardInput{
                vk: key,
                scan_code: 0,
                flags: KEYEVENT_KEYUP,
                time: 0,
                extra_info: 0,
            },
            KeyboardInput{
                vk: KeyCode::Control as u16,
                scan_code: 0,
                flags: KEYEVENT_KEYUP,
                time: 0,
                extra_info: 0,
            }
        ])
    }

}

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 2 {
	println!("Usage: fuzzer.exe <window title to fuzz on>");
	return Ok(());
	}
    let window = Window::attach(&args[1])?;
    println!("Opened a handle to calc!");

    let mut blacklist = HashSet::new();
    blacklist.insert(0x5b); // Left windows key
    blacklist.insert(0x5c); // Right windows key
    blacklist.insert(0x5d); // Application key
    blacklist.insert(0x5f); // Sleep key
    blacklist.insert(0x70); // F1 key
    blacklist.insert(0x73); // F4 key
    blacklist.insert(0x2f); // Help key
    blacklist.insert(0x2a); // Print
    blacklist.insert(0x2b); // Execute
    blacklist.insert(0x1b); // Esc
    blacklist.insert(0x2c); // Print Screen
    blacklist.insert(0x12); // Alt
    blacklist.insert(0x11); // Ctrl

    for key in 0x80..=0xffff{
	blacklist.insert(key);
	}

    loop {
        /// Filter out sending inputs when we're not looking at 'window'
        if unsafe { GetForegroundWindow() } != window.hwnd{
            std::thread::sleep_ms(10);
	    continue;
	 }

	/*let key = window.interesting_keys[
	    window.rand() % window.interesting_keys.len()] as u16;*/
	let key = window.rand() as u8 as u16;

	if blacklist.contains(&key) {
		continue;
	}

	println!("[+] {:#x}",key);

        let sel = window.rand() % 100;
        match sel {
            0 => window.alt_press(key)?,
            1 => window.ctrl_press(key)?,
            _ => window.press(key)?,
        }
    }
    Ok(())
}
