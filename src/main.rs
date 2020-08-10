use structopt::StructOpt;
use serialport::{SerialPortSettings, open_with_settings};
use xmodem::{Xmodem, BlockLength};
use std::fs::{read_to_string};
use std::io::Read;


#[derive(Debug, StructOpt)]
#[structopt(name = "STM32IAPUploader", about = "Uploader for IAP")]
struct Opt{ 
    /// Set port name
    #[structopt(short = "p", long)]
    port: String,

    /// Path to binary
    #[structopt(short = "b", long)]
    binary: String,
    
    /// Baud rate
    #[structopt(short = "r", long)]
    baudrate: u32,
    
    /// MCU Type
    #[structopt(short = "m", long)]
    mcu_type: String,
}
fn main() -> std::io::Result<()> {
    simple_logger::init().unwrap();
    let opt = Opt::from_args();
    println!("Uploading {}", opt.binary);
    println!("Targetting {} @ {}", opt.mcu_type, opt.port);
    let s = SerialPortSettings {
        baud_rate: opt.baudrate,
        ..Default::default()
    };
    println!("Opening port");
    let mut port = open_with_settings(
        &opt.port,
        &SerialPortSettings {
            baud_rate: opt.baudrate,
            timeout: std::time::Duration::from_millis(0),
            ..Default::default()
        }
    )?;
    println!("Opening binary");
    let mut bin = std::fs::File::open(opt.binary)?;
    println!("{:?}", bin);
    // TEST
    // let mut ser_buf = [0u8];
    // loop{
    //     match port.read(&mut ser_buf) {
    //         Ok(_) => print!("{}", ser_buf[0]),
    //         _ => ()
    //     };
    // }
    // Reset the MCU
    println!("Resetting target MCU");
    port.write_data_terminal_ready(true)?;
    port.write_data_terminal_ready(false)?;

    // Wait for BEL
    let mut ser_buf = [0u8];
    println!("Waiting for BEL from MCU");
    loop{ 
        match port.read(&mut ser_buf) {
            Ok(_) => {
                // Respond with ACK
                if ser_buf[0] == 7 {
                    println!("< {}", ser_buf[0]);
                    println!("Received BEL, sending ACK");
                    println!("> {}", 6u8);
                    port.write(&[6u8])?;
                    // std::thread::sleep_ms(100);
                    break;
                }
            },
            _ => ()
        };
    }    
    println!("Waiting for MCU type");
    let mut ser_buf: [u8; 6] = Default::default();
    loop{ 
        match port.read(&mut ser_buf) {
            Ok(_) => {
                let mcu_type = String::from_utf8_lossy(&ser_buf);
                println!("{:?}", ser_buf);
                // Throw error for non matching MCU type and write NAK to serial
                if mcu_type.ne(&opt.mcu_type) {
                    eprintln!("Incorrect MCU target! {} != {}", mcu_type, opt.mcu_type);
                    port.write(&[0x15u8])?;
                    std::process::exit(-1)
                }
                // Write ACK
                port.write(&[6u8])?;
                println!("< {}", mcu_type);
                println!("Correct MCU target {}, sending ACK", opt.mcu_type);
                println!("> {}", 6u8);
                break;
            },
            _ => ()
        };
    }

    let mut ser_buf = [0u8];
    println!("Waiting for MCU...");
    loop{ 
        match port.read(&mut ser_buf) {
            Ok(_) => {
                if ser_buf[0] == 67 {
                    println!("Flashing binary!");
                    break;
                }
            },
            Err(e) => ()//eprintln!("{}", e)
        };
    }
    // Create XMODEM
    let mut xm = Xmodem::new();
    // xm.block_length = BlockLength::OneK;
    match xm.send( &mut port, &mut bin ) {
        Ok(_) => println!("Flashing complete"),
        Err(e) => eprintln!("Error flashing firmware! - {:?}", e)
    }
    Ok(())
}