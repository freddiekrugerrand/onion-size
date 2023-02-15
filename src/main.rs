use lightning::util::ser::{BigSize, Writeable};

fn main() {
    println!("Onion Sizer {}", tlv_size(BigSize(1000), false, 0));
}

// Calculates the total bytes to store the TLVs in a payload, based on the
// presence and values of various TLVs:
// * amount: expressed as a truncated integer, the number of bytes varies with
//   the payment amount (less for smaller, large for larger). We assume away
//   fees, because they're unlikely to significantly change the space
//   requirement (give or take a few edge cases).
// * expiry: for the forseeable future, we'll be under a million blocks, so
//   we expect the truncated expiry height to take (TODO) bytes
// * is_mpp: the final payload will need an extra 32 bytes for a payment_secret
//   and to repeat the payment amount.
// * metadata: arbitrary data for the final hop can have any size.
fn tlv_size(amount: BigSize, is_mpp: bool, metadata_len: usize) -> usize {
    // Start with our payload size and field count assuming that we have a
    // short_channel_id, which is 8 bytes.
    let mut payload_bytes: usize = 8;
    let mut field_count: usize = 1;

    // Encode the amount field so that we know how many truncated bytes it will
    // use, and add it to our field count.
    payload_bytes += amount.encode().len();
    field_count += 1;

    // Use a dummy block height around our current height to calculate the
    // bytes we'll need to include for expiry. As with amount, add
    payload_bytes += BigSize(770_000).encode().len();
    field_count += 1;

    // MPP payments need an extra 32 bytes for a payment_secret and repeat
    // the payment amount in total_msat.
    if is_mpp {
        payload_bytes += 32;
        field_count += 1;
    }

    // If there's non-zero metadata, include it and increment field count.
    if metadata_len != 0 {
        payload_bytes += metadata_len;
        field_count += 1;
    }

    // We need 4 bytes per field for type and length, followed by the total
    // payload bytes we're storing.
    field_count * 4 + payload_bytes
}
