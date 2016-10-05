//
//      fetching data from the Sigma Docking Station TL2012
//      (only tested with a Sigma BC 16.12 unit)
//

// --- configuration

const SDC_VENDOR:    u16 = 0x1d9d;  // TL2012 cradle id
const SDC_PRODUCT:   u16 = 0x1011;

const BC1612:        u8  = 0x15;    // id of the only unit supported in this version

const TIMEOUT_MSEC:  u64 = 6000;    // SDC is quite slow to reply...
const TIMEOUT_SHORT: u64 = 1000;    // ...except when polling


// --- libraries

extern crate libusb;

use std::io::{ Write, stderr };
use std::time::Duration;


// --- macros

macro_rules! msg(             // messages on stderr
    ($($arg:tt)*) => { {
        let _ = writeln!(&mut stderr(), "# {}", format_args!($($arg)*));
    } }
);

macro_rules! hexdump(         // hex dump on stderr
    ($info: expr, $arg: expr, $tot: expr) => { {
        let _ = write!(&mut stderr(), "# {}", $info);
        for i in $arg.iter().take($tot) { write!(&mut stderr(), " {:02x}", i).unwrap(); }
        let _ = writeln!(&mut stderr(), "");
    } }
);

macro_rules! usbytes(         // return if something went wrong on bulk transfers
    ($n:expr, $call:expr) => {
        match $call {
            Err(e) => {
                msg!("bulk transfer error: {}", e);
                return
            },
            Ok(n) => if $n != n {
                msg!("bulk transfer error: expected {} bytes, got {}", $n, n);
                return
            }
        }
    }
);

macro_rules! usbtry(          // return if something went wrong on libusb functions
    ($call:expr) => {
        match $call {
            Ok(_) => (),
            Err(e) => { msg!("libusb error: {}", e); return }
        }
    }
);


// --- common stuff

struct Flags {
    clear:  bool,  // if true, reset unit's counters after successful read
    dump:   bool,  // if true, dump hex data after successful reading info/data packets
    miles:  bool,  // if true, convert kilometers to miles
    ts:     bool,  // if true, output also trip section ts_dist and ts_time fields
    zeros:  bool,  // if true, output also zero-valued fields
    raw:    bool,  // if true, print comma-separated raw field values only
    remove: bool,  // if true, after printing data wait for unit removal from the cradle
    run:    bool   // if true, flags are ok, program can run
}


// --- functions

fn main() {                   // setup flags and run
    let mut flag = Flags {
        clear:  false,
        dump:   false,
        miles:  false,
        ts:     true,
        zeros:  true,
        raw:    false,
        remove: false,
        run:    true
    };

    let args: Vec<String> = std::env::args().collect();
    for arg in args.iter().skip(1) {
        match arg.as_ref() {
            "--clear"       => flag.clear  = true,
            "--dump"        => flag.dump   = true,
            "--miles"       => flag.miles  = true,
            "--no-ts"       => flag.ts     = false,
            "--no-zeros"    => flag.zeros  = false,
            "--raw"         => flag.raw    = true,
            "--wait-remove" => flag.remove = true,
            _               => flag.run    = false
        }
    }

    if flag.run {
        run(&mut flag);
        if flag.run { std::process::exit(1) }  // incomplete run? error 1
    }
    else
    {
        msg!("{}: error: optional command-line arguments are: \
              --clear --dump --miles --no-ts --no-zeros --raw --wait-remove", args[0]);
        std::process::exit(2)
    }
}


fn run(flag: &mut Flags) {    // usb context is only valid here and below
    match libusb::Context::new() {
        Ok(mut context) => {
            match open_device(&mut context) {
                Some((mut dev, desc, mut handle)) => use_device(flag, &mut dev, &desc, &mut handle),
                None => {
                    msg!("could not get device {:04x}:{:04x} - \
                          is it connected? do you need higher privileges?", SDC_VENDOR, SDC_PRODUCT)
                }
            }
        },
        Err(e) => msg!("could not initialize libusb: {}", e)
    }
}


fn open_device(context: &mut libusb::Context)
               -> Option<(libusb::Device, libusb::DeviceDescriptor, libusb::DeviceHandle)> {
    // context.set_log_level(libusb::LogLevel::Info);

    let devices = match context.devices() {
        Ok(d) => d,
        Err(_) => return None
    };

    for mut device in devices.iter() {
        let device_desc = match device.device_descriptor() {
            Ok(d) => d,
            Err(_) => continue
        };

        // don't bother checking if there is more than one cradle - first one, good one
        if device_desc.vendor_id() == SDC_VENDOR && device_desc.product_id() == SDC_PRODUCT {
            match device.open() {
                Ok(handle) => return Some((device, device_desc, handle)),
                Err(_) => continue
            }
        }
    }

    None
}


fn use_device(flag: &mut Flags,
              device: &mut libusb::Device,
              device_desc: &libusb::DeviceDescriptor,
              handle: &mut libusb::DeviceHandle) {
    for n in 0..device_desc.num_configurations() {
        let config_desc = match device.config_descriptor(n) {
            Ok(c) => c,
            Err(_) => continue
        };

        for interface in config_desc.interfaces() {
            for interface_desc in interface.descriptors() {
                for endpoint_desc in interface_desc.endpoint_descriptors() {
                    if endpoint_desc.direction() == libusb::Direction::In &&
                       endpoint_desc.transfer_type() == libusb::TransferType::Bulk {
                        let config  = config_desc.number();
                        let intface = interface_desc.interface_number();
                        let setting = interface_desc.setting_number();

                        match handle.kernel_driver_active(intface) {
                            Ok(true) => handle.detach_kernel_driver(intface).ok(),
                            _ => None
                        };

                        usbtry!(handle.set_active_configuration(config));
                        usbtry!(handle.claim_interface(intface));
                        usbtry!(handle.set_alternate_setting(intface, setting));

                        msg!("cradle found on bus {:03} device {:03}", device.bus_number(), device.address());

                        return mappuoglio(flag, handle)
                    }
                }
            }
        }
    }
}


// real meat comes here
fn mappuoglio(flag: &mut Flags, handle: &mut libusb::DeviceHandle) {
    let send = libusb::request_type(libusb::Direction::Out,
                                    libusb::RequestType::Standard,
                                    libusb::Recipient::Endpoint);
    let recv = libusb::request_type(libusb::Direction::In,
                                    libusb::RequestType::Standard,
                                    libusb::Recipient::Interface);

    let shortimeout  = Duration::from_millis(TIMEOUT_SHORT);
    let writetimeout = Duration::from_millis(TIMEOUT_MSEC);
    let readtimeout  = Duration::from_millis(TIMEOUT_MSEC*2);

    let resetbuf:   [ u8; 2 ] = [ 0xf0, 0x02 ];  // reset counters command
    let mut outbuf: [ u8; 1 ] = [ 0 ];           // single-byte commands
    let mut buf:   [ u8; 32 ] = [ 0; 32 ];       // cradle replies

    let mut notyet = true;

    // a quick polling to check if the cradle channel has to be reset
    outbuf[0] = 0xf4;      // poll command, expecting a single byte 0/1
    match handle.write_bulk(send, &outbuf, shortimeout) {
        Ok(_) => {
            match handle.read_bulk(recv, &mut buf[..], shortimeout) {
                Ok(n) => {
                    if n != 1 {
                        msg!("bulk transfer error: expected 1 byte, got {}", n);
                        return
                    }
                },
                Err(libusb::Error::Timeout) => {
                    msg!("resetting usb channel");
                    usbtry!(handle.reset())
                },
                Err(e) => {
                    msg!("libusb bulk read error: {}", e);
                    return
                }
            }
        },
        Err(libusb::Error::Timeout) => {
            msg!("resetting usb channel");
            usbtry!(handle.reset())
        },
        Err(e) => {
            msg!("libusb bulk write error: {}", e);
            return
        }
    }

    // wait for the unit's presence
    loop {
        outbuf[0] = 0xf4;  // poll command, expecting a single byte 0/1
        usbtry!(handle.write_bulk(send, &outbuf, shortimeout));
        usbytes!(1, handle.read_bulk(recv, &mut buf[..], shortimeout));

        if buf[0] == 1 {
            msg!("unit found in the cradle");
            break
        }

        if notyet {
            msg!("cradle is empty; waiting...");
            notyet = false
        }

        std::thread::sleep(shortimeout)
    }

    outbuf[0] = 0xfe;  // identify command, expecting 11 bytes
    usbtry!(handle.write_bulk(send, &outbuf, writetimeout));
    usbytes!(11, handle.read_bulk(recv, &mut buf[..], readtimeout));

    if flag.dump { hexdump!("identification packet:", buf, 11) }

    // this release expects that buf bytes from 7 to 10 are always zero
    for i in 7..11 {
        if buf[i] > 0 {
            msg!("debug: unexpected values in bytes 7 to 10");
            break
        }
    }

    match buf[1] {
        BC1612 => {
            msg!("unit identified as a BC 16.12 type {} version {}", buf[0], buf[6])
        },
        0 => {
            msg!("unknown unit in the cradle (0), exiting...");
            return
        },
        _ => {
            msg!("warning: unknown unit in the cradle (0x{:02x})", buf[1])
        }
    }

    msg!("unit serial number: {}{}{}{}", buf[2], buf[3], buf[4], buf[5]);

    outbuf[0] = 0xfb;  // fetch data command, expecting 27 bytes
    usbtry!(handle.write_bulk(send, &outbuf, writetimeout));
    usbytes!(27, handle.read_bulk(recv, &mut buf[..], readtimeout));

    if flag.dump  { hexdump!("data packet:", buf, 27) }

    if flag.miles { decode27(buf, flag, "mi", "mph", 1.609344) }
    else          { decode27(buf, flag, "km", "km/hr", 1.0)    }

    if flag.clear {
        // reset unit's counters: a two bytes command, expecting a zero byte
        usbtry!(handle.write_bulk(send, &resetbuf, writetimeout));
        usbytes!(1, handle.read_bulk(recv, &mut buf[..], readtimeout));

        if buf[0] != 0 {
            msg!("counters clearing probably failed (return code {})", buf[0]);
            return
        }

        msg!("non-ts counters cleared");
    }

    if flag.remove { msg!("waiting for unit removal from the cradle") }

    // pat the pet, or wait the unit to be removed
    loop {
        outbuf[0] = 0xf4;  // poll
        usbtry!(handle.write_bulk(send, &outbuf, shortimeout));
        usbytes!(1, handle.read_bulk(recv, &mut buf[..], shortimeout));

        if !flag.remove { break }

        if buf[0] == 0 {
            msg!("unit removed, exiting...");
            break
        }

        std::thread::sleep(shortimeout)
    }

    flag.run = false   // completed: did everything
}


fn decode27(buf: [u8; 32], flag: &mut Flags, udist: &str, uspeed: &str, conv: f64)
{
    if buf[0] > 0 {
        msg!("{} records in the unit; reading the oldest one", buf[0] as usize + 1)
    }

    let mut   dist = (buf[1] as u32)*65536 + (buf[2] as u32)*256 + (buf[3] as u32);
    let    seconds = (buf[4] as u32)*65536 + (buf[5] as u32)*256 + (buf[6] as u32);
    let mut meansp = (buf[7] as u16)*256 + (buf[8] as u16);
    let mut  maxsp = (buf[9] as u16)*256 + (buf[10] as u16);
    let    cadence = buf[11];             // only set if cadence sensor was installed

    // this release expects that buf bytes 12 and 13 are always zero
    if buf[12] != 0 || buf[13] != 0 { msg!("debug: unexpected values in bytes 12 and 13") }

    let mut tsdist = (buf[14] as u32)*65536 + (buf[15] as u32)*256 + (buf[16] as u32);
    let   tseconds = (buf[17] as u32)*65536 + (buf[18] as u32)*256 + (buf[19] as u32);

    // this release expects that buf bytes from 20 to 26 are always zero
    for i in 20..27 {
        if buf[i] > 0 {
            msg!("debug: unexpected values in bytes 20 to 26");
            break
        }
    }

    let      hours = seconds / 3600;
    let    minutes = seconds % 3600;
    let    tshours = tseconds / 3600;
    let     tsmins = tseconds % 3600;

    if conv != 1.0 {
        dist   = ((dist   as f64) / conv) as u32;
        meansp = ((meansp as f64) / conv) as u16;
        maxsp  = ((maxsp  as f64) / conv) as u16;
        tsdist = ((tsdist as f64) / conv) as u32
    }

    if dist > 0 || flag.zeros {
        if flag.raw { print!("{}", dist) }
        else        { println!("distance: {}.{:02} {}", dist / 1000, (dist % 1000) / 10, udist) }
    }
    if flag.raw { print!(",") }

    if seconds > 0 || flag.zeros {
        if flag.raw { print!("{}", seconds) }
        else        { println!("time: {}:{:02}:{:02}", hours, minutes / 60, minutes % 60) }
    }
    if flag.raw { print!(",") }

    if meansp > 0 || flag.zeros {
        if flag.raw { print!("{}.{:02}", meansp / 100, meansp % 100) }
        else        { println!("meanspeed: {}.{:02} {}", meansp / 100, meansp % 100, uspeed) }
    }
    if flag.raw { print!(",") }

    if maxsp > 0 || flag.zeros {
        if flag.raw { print!("{}.{:02}", maxsp / 100, maxsp % 100) }
        else        { println!("maxspeed: {}.{:02} {}", maxsp / 100, maxsp % 100, uspeed) }
    }
    if flag.raw { print!(",") }
  
    if cadence > 0 || flag.zeros {
        if flag.raw { print!("{}", cadence) }
        else        { println!("cadence: {}/min", cadence) }
    }
    if flag.raw { print!(",") }

    if flag.ts {
        if tsdist > 0 || flag.zeros {
            if flag.raw { print!("{}", tsdist) }
            else        { println!("ts_dist: {}.{:02} {}", tsdist / 1000, (tsdist % 1000) / 10, udist) }
        }
        if flag.raw { print!(",") }

        if tseconds > 0 || flag.zeros {
            if flag.raw { print!("{}", tseconds) }
            else        { println!("ts_time: {}:{:02}:{:02}", tshours, tsmins / 60, tsmins % 60) }
        }
    }
    if flag.raw { println!("") }
}

