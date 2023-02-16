use lightning::util::ser::{BigSize, Writeable};
use std::env;

static PAYLOAD_LIMIT: usize = 1300;

fn main() {
    let mut amount = BigSize(1000);
    let mut is_mpp = false;
    let mut metadata = 0;
    let mut extra_payloads: Vec<usize> = Vec::new();

    let args: Vec<String> = env::args().collect();
    let mut i = 0;

    for arg in args.iter() {
        match i {
            0 => {}
            1 => {
                let amount_u64 = arg.to_string().parse().unwrap();
                amount = BigSize(amount_u64);
            }
            2 => {
                is_mpp = match arg.as_ref() {
                    "true" => true,
                    "false" => false,
                    "t" => true,
                    "f" => false,
                    "1" => true,
                    "0" => false,
                    _ => panic!("invalid mpp argument: {}", arg),
                }
            }
            3 => {
                metadata = arg.to_string().parse().unwrap();
            }
            _ => {
                extra_payloads.push(arg.to_string().parse().unwrap());
            }
        }

        i += 1;
    }

    println!(
        "Calculating max hops with amount: {}, mpp: {}, metadata: {}, extra payloads: {}",
        amount.0,
        is_mpp,
        metadata,
        extra_payloads.len(),
    );

    let size = max_hops(amount, is_mpp, metadata, &extra_payloads);
    println!("\nOnion Sizer {} hops / {} filler", size.0, size.1);
}

// Returns the maximum number of hops that will fit in the onion payload and
// the bytes in the filler.
fn max_hops(
    amount: BigSize,
    is_mpp: bool,
    metadata_len: usize,
    extra_payloads: &Vec<usize>,
) -> (usize, usize) {
    // Start with payload for our final hop (which will have mpp and metadata
    // fields).
    let final_payload_tlvs = tlv_size(&amount, is_mpp, metadata_len, extra_payloads);
    let final_payload_total = payload_size(final_payload_tlvs);

    println!("Final hop bytes: {}", final_payload_total);

    // On the off chance that our final payload exceeds the limit, we can't
    // have any hops.
    if final_payload_total > PAYLOAD_LIMIT {
        return (0, PAYLOAD_LIMIT);
    }

    // For intermediate hops, we don't include any mpp fields or metadata.
    let intermediate_payload_tlvs = tlv_size(&amount, false, 0, extra_payloads);
    let intermediate_payload_total = payload_size(intermediate_payload_tlvs);

    let available_bytes = PAYLOAD_LIMIT - final_payload_total;
    let intermediate_hops = available_bytes / intermediate_payload_total;

    println!(
        "Intermediate hops: {} bytes: {}",
        intermediate_hops, intermediate_payload_total
    );

    // Our hop count is 1 + intermediate hops, with
    (
        1 + intermediate_hops,
        available_bytes - (intermediate_hops * intermediate_payload_total),
    )
}

fn payload_size(tlv_total: usize) -> usize {
    // The total size for an individual payload is:
    // - A BigSize for the length of the payload.
    // - All the TLV fields.
    // - 32 bytes for a hmac.
    let with_hmac = tlv_total + 32;
    let len_bytes = BigSize(with_hmac.try_into().unwrap()).encode().len();

    with_hmac + len_bytes
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
// * extra_payloads: a vector of additional payload lengths, intended to
//   capture the size requirements for any new fields added to the onion.
fn tlv_size(
    amount: &BigSize,
    is_mpp: bool,
    metadata_len: usize,
    extra_payloads: &Vec<usize>,
) -> usize {
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

    // If there are any extra payloads included in the hop, add each of their
    // lengths.
    for p in extra_payloads {
        payload_bytes += p;
        field_count += 1;
    }

    // We need 4 bytes per field for type and length, followed by the total
    // payload bytes we're storing.
    field_count * 4 + payload_bytes
}
