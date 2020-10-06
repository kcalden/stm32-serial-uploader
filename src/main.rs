use structopt::StructOpt;
use serialport::{SerialPortSettings, open_with_settings};
use xmodem::{Xmodem, BlockLength};
use std::fs::{read_to_string};
use std::io::Read;

const BAUDRATE: u32 = 250000;
// const DTR_TOGGLES: u32 = 2;
#[derive(Debug, StructOpt)]
#[structopt(name = "STM32IAPUploader", about = "Uploader for IAP")]
struct Opt{ 
    /// Set port name
    #[structopt(short = "p", long)]
    port: String,

    /// Path to binary
    #[structopt(short = "f", long)]
    binary: String,
    
    /// Baud rate
    // #[structopt(short = "r", long)]
    // baudrate: u32,
    
    /// MCU Type
    #[structopt(short = "m", long)]
    mcu_type: String,
}

fn main() -> std::io::Result<()> {
    simple_logger::init().unwrap();
    let opt = Opt::from_args();
    println!("Uploading {}", opt.binary);
    println!("Targetting {} @ {}", opt.mcu_type, opt.port);

    // Open serial port
    println!("Opening port");
    let mut port = open_with_settings(
        &opt.port,
        &SerialPortSettings {
            baud_rate: BAUDRATE,
            timeout: std::time::Duration::from_secs(1),
            ..Default::default()
        }
    )?;

    // Open binary
    println!("Opening binary");
    let mut bin = std::fs::File::open(opt.binary)?;
    println!("{:?}", bin);

    let mut retries_left = 3;
    let mut validation_success = false;
    while retries_left > 0 && !validation_success {
        // Reset the MCU
        println!("Resetting target MCU");
        // for _i in 1..DTR_TOGGLES {
        //     std::thread::sleep(std::time::Duration::from_millis(100));
        //     port.write_data_terminal_ready(false)?;
        //     port.write_data_terminal_ready(true)?;
        // }
        
        // Wait for BEL
        let mut ser_buf = [0u8];
        println!("Waiting for BEL from MCU");
        loop{ 
            port.write_data_terminal_ready(true)?;
            port.write_data_terminal_ready(false)?;
            match port.read(&mut ser_buf) {
                Ok(_) => {
                    // Respond with ACK
                    if ser_buf[0] == 7 {
                        println!("< {}", ser_buf[0]);
                        println!("Received BEL, sending ACK");
                        println!("> {}", 6u8);
                        port.write(&[6u8])?;
                        break;
                    }
                },
                Err(e) => eprintln!("{}",e)
            };
        }
        
        // Check MCU type
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
                        eprintln!("Retries left: {}", retries_left);
                        port.write(&[0x15u8])?;
                        // std::process::exit(-1)
                    }else{
                        // Write ACK
                        port.write(&[6u8])?;
                        println!("< {}", mcu_type);
                        println!("Correct MCU target {}, sending ACK", opt.mcu_type);
                        println!("> {}", 6u8);
                        validation_success = true;
                        break;
                    }
                },
                Err(e) => eprintln!("{}", e)
            };
        }
        if validation_success { break; }
        retries_left -= 1;
    }
    if !validation_success {
        eprint!("MCU validation fail.");
        std::process::exit(0);
    }

    // Wait for XMODEM transfer
    // This will error a few times. At this point the processor is clearing it's flash memory
    let mut ser_buf = [0u8];
    println!("Waiting for MCU to clear...");
    loop{ 
        match port.read(&mut ser_buf) {
            Ok(_) => {
                if ser_buf[0] == 67 {
                    println!("Flashing binary!");
                    break;
                }
            },
            Err(_e) =>  ()// eprintln!("{}", e)
        };
    }
    // Create XMODEM
    let mut xm = Xmodem::new();
    xm.max_errors = 6;
    // xm.block_length = BlockLength::OneK;
    match xm.send( &mut port, &mut bin ) {
        Ok(_) => println!("Flashing complete"),
        Err(e) => eprintln!("Error flashing firmware! - {:?}", e)
    }
    Ok(())
}