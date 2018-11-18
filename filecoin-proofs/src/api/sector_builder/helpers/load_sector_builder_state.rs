use api::sector_builder::kv_store::KeyValueStore;
use api::sector_builder::state::SectorBuilderState;
use error::Result;

pub fn load_sector_builder_state(
    kv_store: &KeyValueStore,
    prover_id: [u8; 31],
) -> Result<Option<SectorBuilderState>> {
    let result: Option<Vec<u8>> = kv_store.get(&prover_id[..])?;

    if let Some(val) = result {
        return serde_cbor::from_slice(&val[..])
            .map_err(failure::Error::from)
            .map(Option::Some);
    }

    Ok(None)
}
