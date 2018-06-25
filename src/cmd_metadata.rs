use std::fs;
use std::ffi::OsStr;
use std::error::Error;
use std::collections::HashMap;

use serde_json;

use archive::{Packet, ArchiveReader};
use metadata::{self, Metadata};

pub struct Args< 'a > {
    pub input_path: &'a OsStr
}

pub fn main( args: Args ) -> Result< (), Box< Error > > {
    let fp = fs::File::open( args.input_path ).map_err( |err| format!( "cannot open {:?}: {}", args.input_path, err ) )?;
    let mut reader = ArchiveReader::new( fp ).validate_header().unwrap().skip_unknown();

    let mut binary_id_to_index = HashMap::new();
    let mut is_valid = false;
    let mut metadata = Metadata::default();

    while let Some( packet ) = reader.next() {
        let packet = packet.unwrap();
        is_valid = true;

        match packet {
            Packet::MachineInfo { architecture, .. } => {
                metadata.machine_info = Some( metadata::MachineInfo { architecture: architecture.into() } );
            },
            Packet::ProcessInfo { pid, executable, .. } => {
                metadata.processes.push( metadata::Process {
                    pid,
                    executable: String::from_utf8_lossy( &executable ).into()
                });
            },
            Packet::BinaryInfo { id, path, debuglink, .. } => {
                let path = String::from_utf8_lossy( &path ).into_owned();

                let debuglink_length = debuglink.iter().position( |&byte| byte == 0 ).unwrap_or( debuglink.len() );
                let debuglink = &debuglink[ 0..debuglink_length ];
                let debuglink = if debuglink.is_empty() {
                    None
                } else {
                    Some( String::from_utf8_lossy( &debuglink ).into_owned() )
                };

                binary_id_to_index.insert( id, metadata.binaries.len() );
                metadata.binaries.push( metadata::Binary {
                    path,
                    debuglink,
                    build_id: None
                });
            },
            Packet::BuildId { id, build_id } => {
                if let Some( &index ) = binary_id_to_index.get( &id ) {
                    let build_id: Vec< _ > = build_id.iter().map( |byte| format!( "{:02x}", byte ) ).collect();
                    let build_id = build_id.join( "" );
                    metadata.binaries[ index ].build_id = Some( build_id );
                }
            },
            _ => {}
        }
    }

    if !is_valid {
        return Err( format!( "input {:?} is not a valid archive", args.input_path ).into() )
    }

    println!( "{}", serde_json::to_string_pretty( &metadata ).unwrap() );
    Ok(())
}