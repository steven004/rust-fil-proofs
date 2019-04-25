use std::sync::Arc;

use crate::api::sector_builder::errors::*;
use crate::api::sector_builder::metadata::sum_piece_bytes;
use crate::api::sector_builder::metadata::StagedSectorMetadata;
use crate::api::sector_builder::state::StagedState;
use crate::api::sector_builder::*;
use crate::error;
use sector_base::api::bytes_amount::UnpaddedBytesAmount;
use sector_base::api::sector_store::SectorManager;
use sector_base::api::SectorId;

pub fn add_piece(
    sector_store: &Arc<WrappedSectorStore>,
    mut staged_state: &mut StagedState,
    piece_key: String,
    piece_bytes: &[u8],
) -> error::Result<SectorId> {
    let sector_mgr = sector_store.inner.manager();
    let sector_max = sector_store.inner.config().max_unsealed_bytes_per_sector();

    let piece_bytes_len = UnpaddedBytesAmount(piece_bytes.len() as u64);

    staged_state.sector_id_nonce = get_sectorid_from_cid(&piece_key)?;

    // TO DO: just use a new access for a new piece of data. 
    let opt_dest_sector_id = {
        let candidates: Vec<StagedSectorMetadata> = staged_state
            .sectors
            .iter()
            .filter(|(_, v)| v.seal_status == SealStatus::Pending)
            .map(|(_, v)| (*v).clone())
            .collect();

        compute_destination_sector_id(&candidates[..], sector_max, piece_bytes_len)?
    };

    let dest_sector_id = opt_dest_sector_id
        .ok_or(())
        .or_else(|_| provision_new_staged_sector(sector_mgr, &mut staged_state))?;

    // To use determined sector_id based on the piece_key, and already create a new sector
    // let dest_sector_id = provision_new_staged_sector(sector_mgr, &mut staged_state), piece_key)?;

    if let Some(s) = staged_state.sectors.get_mut(&dest_sector_id) {
        sector_store
            .inner
            .manager()
            .write_and_preprocess(&s.sector_access, &piece_bytes)
            .map_err(Into::into)
            .and_then(|num_bytes_written| { 
                if num_bytes_written != piece_bytes_len {
                    Err(
                        err_inc_write(u64::from(num_bytes_written), u64::from(piece_bytes_len))
                            .into(),
                    )
                } else {
                    Ok(s.sector_id)
                }
            })
            .map(|sector_id| {
                s.pieces.push(metadata::PieceMetadata {
                    piece_key, 
                    num_bytes: piece_bytes_len,
                });

                sector_id
            })
    } else {
        Err(err_unrecov("unable to retrieve sector from state-map").into())
    }
}

// Given a list of staged sectors which are accepting data, return the
// first staged sector into which the bytes will fit.
fn compute_destination_sector_id(
    candidate_sectors: &[StagedSectorMetadata],
    max_bytes_per_sector: UnpaddedBytesAmount,
    num_bytes_in_piece: UnpaddedBytesAmount,
) -> error::Result<Option<SectorId>> {
    if num_bytes_in_piece > max_bytes_per_sector {
        Err(err_overflow(num_bytes_in_piece.into(), max_bytes_per_sector.into()).into())
    } else {
        Ok(candidate_sectors
            .iter()
            .find(move |staged_sector| {
                (max_bytes_per_sector - sum_piece_bytes(staged_sector)) >= num_bytes_in_piece
            })
            .map(|x| x.sector_id))
    }
}

// Provisions a new staged sector and returns its sector_id. Not a pure
// function; creates a sector access (likely a file), increments the sector id
// nonce, and mutates the StagedState.
fn provision_new_staged_sector(
    sector_manager: &SectorManager,
    staged_state: &mut StagedState,
) -> error::Result<SectorId> {
    // // Do not use the original increamental sector_id
    // let sector_id = {
    //     let n = &mut staged_state.sector_id_nonce;
    //     *n += 1;
    //     *n
    // };

    // To use a determined sector_id
    // let cid_b = piece_key.as_bytes();
    // let l = cid_b.len();
    // if l < 8 {
    //     return Err("The length of ths tring is less than 8");
    // }

    // // println!("cid = { }", cid);   
    // let mut sector_id:SectorId = 0;
    // for i in l-8..l {
    //     sector_id <<= 8;
    //     sector_id += cid_b[i] as u64;
    // }
    // staged_state.sector_id_nonce = sector_id;

    let sector_id = staged_state.sector_id_nonce;
    let access = sector_manager.new_staging_sector_access(sector_id)?;

    let meta = StagedSectorMetadata {
        pieces: Default::default(),
        sector_access: access.clone(),
        sector_id,
        seal_status: SealStatus::Pending,
    };

    staged_state.sectors.insert(meta.sector_id, meta.clone());

    Ok(sector_id)
}

// Get a determined sector_id from a cid
fn get_sectorid_from_cid(cid: &str) -> error::Result<SectorId> { //? String
    let cid_b = cid.as_bytes();
    let l = cid_b.len();
    if l < 8 {
        return Err(format_err!("The length of ths tring is less than 8")); //?
        // return Err("The length of ths tring is less than 8");
    }

    // println!("cid = { }", cid);   
    let mut sector_id:SectorId = 0;
    for i in l-8..l {
        sector_id <<= 8;
        sector_id += cid_b[i] as u64;
    }
    
    Ok(sector_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::sector_builder::metadata::PieceMetadata;

    #[test]
    fn test_alpha() {
        let mut sealed_sector_a: StagedSectorMetadata = Default::default();

        sealed_sector_a.pieces.push(PieceMetadata {
            piece_key: String::from("x"),
            num_bytes: UnpaddedBytesAmount(5),
        });

        sealed_sector_a.pieces.push(PieceMetadata {
            piece_key: String::from("x"),
            num_bytes: UnpaddedBytesAmount(10),
        });

        let mut sealed_sector_b: StagedSectorMetadata = Default::default();

        sealed_sector_b.pieces.push(PieceMetadata {
            piece_key: String::from("x"),
            num_bytes: UnpaddedBytesAmount(5),
        });

        let staged_sectors = vec![sealed_sector_a.clone(), sealed_sector_b.clone()];

        // piece takes up all remaining space in first sector
        match compute_destination_sector_id(
            &staged_sectors,
            UnpaddedBytesAmount(100),
            UnpaddedBytesAmount(85),
        ) {
            Ok(Some(destination_sector_id)) => {
                assert_eq!(destination_sector_id, sealed_sector_a.sector_id)
            }
            _ => panic!(),
        }

        // piece doesn't fit into the first, but does the second
        match compute_destination_sector_id(
            &staged_sectors,
            UnpaddedBytesAmount(100),
            UnpaddedBytesAmount(90),
        ) {
            Ok(Some(destination_sector_id)) => {
                assert_eq!(destination_sector_id, sealed_sector_b.sector_id)
            }
            _ => panic!(),
        }

        // piece doesn't fit into any in the list
        match compute_destination_sector_id(
            &staged_sectors,
            UnpaddedBytesAmount(100),
            UnpaddedBytesAmount(100),
        ) {
            Ok(None) => (),
            _ => panic!(),
        }

        // piece is over max
        match compute_destination_sector_id(
            &staged_sectors,
            UnpaddedBytesAmount(100),
            UnpaddedBytesAmount(101),
        ) {
            Err(_) => (),
            _ => panic!(),
        }
    }
}
