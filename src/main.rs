extern crate rand;
extern crate libusb;

use std::str;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::fs;
use std::fs::File;
use std::time::Duration;
use std::path::PathBuf;
use std::u8;
use rand::Rng;


/*
 * sets and reads fan and pump speeds
 */
struct UsbController<'a> {
    handle: libusb::DeviceHandle<'a>,
    interface: u8,
    read_address: u8,
    write_address: u8
}


impl<'a> UsbController<'a> {
    fn open(device: &'a libusb::Device) -> UsbController<'a> {

        let mut selected_interface = 0x00;
        let mut selected_read_address = 0x81;
        let mut selected_write_address = 0x01;

        let config = device.active_config_descriptor().unwrap();
        for interface in config.interfaces() {
            selected_interface = interface.number();

            for descriptor in interface.descriptors() {

                for endpoint in descriptor.endpoint_descriptors() {
                    if endpoint.direction() == libusb::Direction::In {
                        selected_read_address = endpoint.address();
                    }
                    else {
                        selected_write_address = endpoint.address();
                    }
                }
            }
        }

        println!("Opening interface 0x{:02x}", selected_interface);
        println!("Read address 0x{:02x}", selected_read_address);
        println!("Write address 0x{:02x}", selected_write_address);

        return UsbController {
            handle: device.open().unwrap(),
            interface: selected_interface,
            read_address: selected_read_address,
            write_address: selected_write_address,
        }
    }

    fn claim(&mut self) {
        self.handle.detach_kernel_driver(self.interface);
        self.handle.claim_interface(self.interface);
    }

    fn release(&mut self) {
        self.handle.release_interface(self.interface);
    }


    fn set_color(&mut self, r: u8, g: u8, b: u8) {
        let mode = 4; // 3

        self.write_init_commands();
        self.write_color_command(0, 2, mode, 0, 127, 255, 255, 127, 0);
        self.write_color_command(2, 2, mode, 0, 127, 255, 255, 127, 0);
        self.write_color_command(4, 2, mode, 0, 127, 255, 255, 127, 0);
        self.write_color_command(6, 2, mode, 0, 127, 255, 255, 127, 0);
        self.write_color_command(8, 2, mode, 0, 127, 255, 255, 127, 0);
        self.write_color_command(10, 2, mode, 0, 127, 255, 255, 127, 0);
        self.write_color_command(12, 2, mode, 0, 127, 255, 255, 127, 0);
        self.write_color_command(14, 2, mode, 0, 127, 255, 255, 127, 0);
        self.write_final_command();
    }

    fn write_init_commands(&mut self) {
        let msgs: [[u8; 3]; 1] = [
                //[0x37, 0x01, 0x00],
                //[0x34, 0x01, 0x00],
                //[0x38, 0x01, 0x01],
                [0x37, 0x00, 0x00],
                //[0x34, 0x00, 0x00],
                //[0x38, 0x00, 0x01],
            ];

        for buf in msgs.iter() {
            self.send_msg(buf);
        }
    }

    /**
     * offset: led index
     * modes: 0x01 = fade, 0x04 = static, 0x06 = sequence, 0x08 = blink
     */
    fn write_color_command(&mut self, offset: u8, count: u8, mode: u8, r1: u8, g1: u8, b1: u8, r2: u8, g2: u8, b2: u8) {
        let speed = 0x02; // 0x00 fastest, 0x01 medium, 0x02 slowest
        let sequence = 0x01; // 0x00 or 0x01 for random

        let buf: [u8; 24] = [
                0x35, 0x00, offset, count, mode, speed, sequence, 0x00,
                0xff, r1, g1, b1, r2, g2, b2, 0xff,
                0x00, 0x00, 0x09, 0xc4, 0x0d, 0xac, 0x11, 0x94
            ];

        self.send_msg(&buf);
    }

    fn write_final_command(&mut self) {
        let buf: [u8; 2] = [0x33, 0xff];
        self.send_msg(&buf);
    }



    fn send_msg(&mut self, msg: &[u8]) {
        let result = self.handle.write_interrupt(self.write_address, msg, Duration::from_secs(1)).unwrap();
        self.print_status();
    }


    fn print_status(&mut self) {
        let mut buf: [u8; 64] = [0; 64];
        let result = self.handle.read_interrupt(self.read_address, &mut buf, Duration::from_secs(1)).unwrap();
        println!("read {:03} bytes", result);
    }
}


fn print_endpoint(endpoint: libusb::EndpointDescriptor) {
    println!("Endpoint address {:02x}", endpoint.address());
    println!("Endpoint number {:02x}", endpoint.number());
    println!("Endpoint direction {:?}", endpoint.direction());
    println!("Endpoint transfer {:?}", endpoint.transfer_type());
    println!("Endpoint sync {:?}", endpoint.sync_type());
    println!("Endpoint usage {:?}", endpoint.usage_type());
    println!("Endpoint packet size {}", endpoint.max_packet_size());
}


fn print_device(device: &libusb::Device) {
    let device_desc = device.device_descriptor().unwrap();
    println!("Bus {:03} Device {:03} ID {:04x}:{:04x}",
        device.bus_number(),
        device.address(),
        device_desc.vendor_id(),
        device_desc.product_id());

    let config = device.active_config_descriptor().unwrap();
    println!("Number {}, Interfaces {}", config.number(), config.num_interfaces());

    for interface in config.interfaces() {
        println!("Interface {:04x}", interface.number());
        for descriptor in interface.descriptors() {
            println!("Endpoints {}", descriptor.num_endpoints());
            for endpoint in descriptor.endpoint_descriptors() {
                print_endpoint(endpoint);
            }
        }
    }
}


fn select_device(device: libusb::Device) {

    // print all device information
    print_device(&device);

    let mut controller = UsbController::open(&device);
    controller.claim();

    controller.set_color(255, 0, 0);

    controller.release();
}


fn main() {
    // usb id
    let vendor_id = 0x1b1c;
    let product_id = 0x0c0b;
    let mut context = libusb::Context::new().unwrap();

    // device selection
    for mut device in context.devices().unwrap().iter() {
        let device_desc = device.device_descriptor().unwrap();
        if device_desc.vendor_id() == vendor_id && device_desc.product_id() == product_id {
            select_device(device);
        }
    }
}
