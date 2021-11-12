#![allow(non_snake_case)]
#[macro_use]

extern crate dlopen_derive;
extern crate dlopen;
extern crate libc;

use dlopen::{wrapper::{Container, WrapperApi}};
use libc::{c_int, c_double};
use libc::{malloc};
use structopt::StructOpt;

#[derive(WrapperApi)]
struct Manipulator<> {
    LV_GetCountAxisOfManip: unsafe extern "stdcall" fn() -> c_int,
    LV_InitializeManip: unsafe extern "stdcall" fn() -> c_int,
    LV_MoveToAxis: unsafe extern "stdcall" fn(axis: c_int, pos: c_double) -> c_int,
    LV_MoveToAxisVelocity: unsafe extern "stdcall" fn(axis: c_int, pos: c_double, speed: c_double) -> c_int,
    LV_ReadPosManip: unsafe extern "stdcall" fn(pos: *mut c_double, speed: *mut c_double) -> c_int,
    LV_StatusManip: unsafe extern "stdcall" fn() -> c_int,
    LV_StopManip: unsafe extern "stdcall" fn() -> c_int,
}

#[derive(Debug, StructOpt)]
#[structopt(name = "manip", about = "PreVac Manipulator Control")]
enum Cli {
    /// Move a specified axis to a position
    Move {
        /// Axis
        #[structopt(short, long)]
        axis: i32,

        /// Position
        #[structopt(short, long)]
        position: f64,
    },

    /// Move a specified axis to a position with specified speed
    MoveSpeed {
        /// Axis
        #[structopt(short, long)]
        axis: i32,

        /// Position
        #[structopt(short, long)]
        position: f64,

        /// Speed
        #[structopt(short, long)]
        speed: f64,
    },

    /// Get the positions and speeds of all axes
    Position {},

    /// Get the number of axes
    NumAxes {},

    /// Get the status of the manipulator
    // returns a number
    //  0: moving
    //  1: done
    //  2: aborted
    //  3: error
    //  4: no status
    Status {},

    /// Stop manipulator
    Stop {},
}

fn main () {
    let container: Container<Manipulator> = unsafe { Container::load("ManipulatorDLL.dll")}.unwrap();

    // Initialize the manipulator library. A 0 seems to indicate that the initialization was successful.
    match initialize(&container) {
        Ok(_) => (),
        Err(ret) => panic!("Error while initializing. Return Code: {}", ret)
    };

    let opt = Cli::from_args();

    let ret: Result<(), i32> = match opt {
        Cli::Move { axis, position } => 
            match move_axis(&container, axis, position) {
                Ok(_) => Ok(()),
                Err(ret) => panic!("error: {}", ret),
            },

        Cli::MoveSpeed { axis, position, speed } => 
            match move_axis_with_speed(&container, axis, position, speed) {
                Ok(_) => Ok(()),
                Err(ret) => panic!("error: {}", ret),
            },

        Cli::Position {} => {
            let num_axes = get_num_axes(&container);
            let (pos, speed) = match get_pos_and_speed(&container, num_axes) {
                Ok(val) => val,
                Err(ret) => panic!("error: {}", ret)
            };
            print_pos_and_speed(pos, speed);
            Ok(())
        },

        Cli::NumAxes {} => {
            let num_axes = get_num_axes(&container);
            println!("{}", num_axes);
            Ok(())
        },

        Cli::Status {} => {
            let status = get_status(&container);
            println!("{}", status);
            Ok(())
        },

        Cli::Stop {} => {
            match stop(&container) {
                Ok(_) => Ok(()),
                Err(ret) => panic!("Error stopping the manipulator: {}", ret)
            }
        },
    };
    // not necessary but otherwise there's a warning in the above block
    drop(ret);

    //symbols are released together with library handle
    //this prevents dangling symbols
    drop(container);
}

fn print_pos_and_speed(pos: Vec<f64>, speed: Vec<f64>) {
    print!("pos: ");
    for p in pos {
        print!("{} ", p);
    }

    print!("\n");

    print!("speed: ");
    for s in speed {
        print!("{} ", s);
    }

    print!("\n")
}

fn initialize(container: &Container<Manipulator>) -> Result<i32, i32> {
    let ret = unsafe {
        container.LV_InitializeManip()
    };

    match ret {
        0 => Ok(ret),
        _ => Err(ret)
    }
}

fn get_status(container: &Container<Manipulator>) -> i32 {
    let ret = unsafe {
        container.LV_StatusManip()
    };

    ret
}

fn move_axis(container: &Container<Manipulator>, axis: i32, pos: f64) -> Result<i32, i32> {
    let ret = unsafe {
        container.LV_MoveToAxis(axis, pos)
    };

    match ret {
        0 => Ok(ret),
        _ => Err(ret)
    }
}

fn move_axis_with_speed(container: &Container<Manipulator>, axis: i32, pos: f64, speed: f64) -> Result<i32, i32> {
    let ret = unsafe {
        container.LV_MoveToAxisVelocity(axis, pos, speed)
    };

    match ret {
        0 => Ok(ret),
        _ => Err(ret)
    }
}

fn get_num_axes(container: &Container<Manipulator>) -> usize {
    let num_axes = unsafe {
        container.LV_GetCountAxisOfManip() as usize
    };

    num_axes
}

fn stop(container: &Container<Manipulator>) -> Result<i32, i32> {
    let ret = unsafe {
        container.LV_StopManip()
    };

    match ret {
        0 => Ok(ret),
        _ => Err(ret)
    }
}

fn get_pos_and_speed(container: &Container<Manipulator>, num_axes: usize) -> Result<(Vec<f64>, Vec<f64>), i32> {
    
    // no idea what this is doing. But doing it this way gives a STATUS_HEAP_CORRUPTION error from time to time
    // can't put the next two blocks into a function
    // let pos = vec![0.0; num_axes];
    // let speed = vec![0.0; num_axes];
    // let ptr_pos = Box::into_raw(Box::new(pos)) as *mut f64;
    // let ptr_speed = Box::into_raw(Box::new(speed)) as *mut f64;
    // it's working this way, however
    let size = std::mem::size_of::<c_double>() * num_axes;
    let ptr_pos: *mut f64 = unsafe { malloc(size) as *mut f64 };
    let ptr_speed: *mut f64 = unsafe { malloc(size) as *mut f64 };

    let ret = unsafe { container.LV_ReadPosManip(ptr_pos, ptr_speed) };

    // no idea how to put this into a function
    let speed: Vec<f64> = unsafe {
        assert!(!ptr_speed.is_null());

        let slice = std::slice::from_raw_parts(ptr_speed, num_axes);
        Vec::<f64>::from(slice)
    };

    // no idea how to put this into a function
    let pos: Vec<f64> = unsafe {
        assert!(!ptr_pos.is_null());

        let slice = std::slice::from_raw_parts(ptr_pos, num_axes);
        Vec::<f64>::from(slice)
    };

    match ret {
        0 => Ok((pos, speed)),
        _ => Err(ret)
    }
}

