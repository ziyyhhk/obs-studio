//! Packed UYVX pixel block helpers for the format-conversion kernels.
//!
//! Each helper converts one block of four UYVX pixels (16 bytes per row) into
//! its luma and chroma outputs, mirroring the C's SSE2 block. On `x86_64` the
//! SSE2 path runs unconditionally: SSE2 is part of the x86_64 baseline, so the
//! aligned load the C relies on is not a requirement here (`_mm_loadu_si128`),
//! and the integer ops are the same ones the C uses. Other hosts run the same
//! arithmetic byte for byte in scalar form, through the `channel`/`average`
//! helpers of the safe core. The test oracle compiles the C both ways, so the
//! parity tests compare all of them bit for bit.
//!
//! The luma pack is the C's `pack_shift`: mask the Y byte of each pixel with
//! `0x0000FF00`, shift the 128-bit right by `sh` bytes (1 for Y, 2 for V),
//! pack the two rows' dwords to 16 bits with saturating
//! `_mm_packs_epi32`, then to bytes with `_mm_packus_epi16`. The chroma pack
//! is the C's `pack_ch_1plane`/`pack_ch_2plane`: `_mm_add_epi64` over the
//! `0x00FF`-masked rows, a shuffle pair to sum adjacent dword lanes, an
//! arithmetic `_mm_srai_epi16(..., 2)`, and (2plane only) an extra
//! `_mm_shufflelo_epi16` to split U from V before the final pack.

#![allow(unsafe_code)]

#[cfg(not(target_arch = "x86_64"))]
use crate::format_conversion::{UYVX_U, UYVX_V, UYVX_Y, average, channel};

/// `compress_uyvx_to_i420`: one block of the luma row pair and the U and V
/// planes; `block` is the byte offset of the block in the read rows, `pixel`
/// the pixel offset (`i * 4`).
#[cfg(target_arch = "x86_64")]
#[inline]
pub(crate) fn uyvx_block_i420(
    luma: &mut [u8],
    lum_stride: usize,
    u_row: &mut [u8],
    v_row: &mut [u8],
    line1: &[u8],
    line2: &[u8],
    block: usize,
    i: usize,
) {
    // SAFETY: SSE2 is part of the x86_64 baseline; `block` is a multiple of
    // 16 and every slice covers the bytes the C touches.
    unsafe { block_i420_sse2(luma, lum_stride, u_row, v_row, line1, line2, block, i) };
}

/// `compress_uyvx_to_i420` on hosts without SSE2.
#[cfg(not(target_arch = "x86_64"))]
#[inline]
pub(crate) fn uyvx_block_i420(
    luma: &mut [u8],
    lum_stride: usize,
    u_row: &mut [u8],
    v_row: &mut [u8],
    line1: &[u8],
    line2: &[u8],
    block: usize,
    i: usize,
) {
    let luma_at = i * 4;
    luma[luma_at..luma_at + 4].copy_from_slice(&channel(line1, block, UYVX_Y));
    luma[lum_stride + luma_at..lum_stride + luma_at + 4]
        .copy_from_slice(&channel(line2, block, UYVX_Y));
    let [u01, u23] = average(line1, line2, block, UYVX_U);
    let [v01, v23] = average(line1, line2, block, UYVX_V);
    u_row[i * 2..i * 2 + 2].copy_from_slice(&[u01, u23]);
    v_row[i * 2..i * 2 + 2].copy_from_slice(&[v01, v23]);
}

/// `compress_uyvx_to_nv12`: one block of the luma row pair and the
/// interleaved U V chroma row.
#[cfg(target_arch = "x86_64")]
#[inline]
pub(crate) fn uyvx_block_nv12(
    luma: &mut [u8],
    lum_stride: usize,
    chroma_row: &mut [u8],
    line1: &[u8],
    line2: &[u8],
    block: usize,
    i: usize,
) {
    // SAFETY: as above.
    unsafe { block_nv12_sse2(luma, lum_stride, chroma_row, line1, line2, block, i) };
}

/// `compress_uyvx_to_nv12` on hosts without SSE2.
#[cfg(not(target_arch = "x86_64"))]
#[inline]
pub(crate) fn uyvx_block_nv12(
    luma: &mut [u8],
    lum_stride: usize,
    chroma_row: &mut [u8],
    line1: &[u8],
    line2: &[u8],
    block: usize,
    i: usize,
) {
    let pixel = i * 4;
    luma[pixel..pixel + 4].copy_from_slice(&channel(line1, block, UYVX_Y));
    luma[lum_stride + pixel..lum_stride + pixel + 4].copy_from_slice(&channel(line2, block, UYVX_Y));
    let [u01, u23] = average(line1, line2, block, UYVX_U);
    let [v01, v23] = average(line1, line2, block, UYVX_V);
    chroma_row[pixel..pixel + 4].copy_from_slice(&[u01, v01, u23, v23]);
}

/// `convert_uyvx_to_i444`: one block of all three plane row pairs.
#[cfg(target_arch = "x86_64")]
#[inline]
pub(crate) fn uyvx_block_i444(
    luma: &mut [u8],
    u_row: &mut [u8],
    v_row: &mut [u8],
    stride: usize,
    line1: &[u8],
    line2: &[u8],
    block: usize,
    i: usize,
) {
    // SAFETY: as above.
    unsafe { block_i444_sse2(luma, u_row, v_row, stride, line1, line2, block, i) };
}

/// `convert_uyvx_to_i444` on hosts without SSE2.
#[cfg(not(target_arch = "x86_64"))]
#[inline]
pub(crate) fn uyvx_block_i444(
    luma: &mut [u8],
    u_row: &mut [u8],
    v_row: &mut [u8],
    stride: usize,
    line1: &[u8],
    line2: &[u8],
    block: usize,
    i: usize,
) {
    let pixel = i * 4;
    for (rows, c) in [
        (&mut *luma, UYVX_Y),
        (&mut *u_row, UYVX_U),
        (&mut *v_row, UYVX_V),
    ] {
        rows[pixel..pixel + 4].copy_from_slice(&channel(line1, block, c));
        rows[stride + pixel..stride + pixel + 4].copy_from_slice(&channel(line2, block, c));
    }
}

#[cfg(target_arch = "x86_64")]
mod sse2 {
    use core::arch::x86_64::{
        __m128i, _mm_add_epi64, _mm_and_si128, _mm_cvtsi128_si32, _mm_loadu_si128,
        _mm_packs_epi32, _mm_packus_epi16, _mm_set1_epi16, _mm_set1_epi32, _mm_shuffle_epi32,
        _mm_shufflelo_epi16, _mm_srai_epi16, _mm_srli_si128,
    };

    const SHUFFLE_D1: i32 = 0b1011_0001; // _MM_SHUFFLE(2, 3, 0, 1)
    const SHUFFLE_D8: i32 = 0b1101_1000; // _MM_SHUFFLE(3, 1, 2, 0)

    #[inline]
    pub(super) unsafe fn mask_loadu(line: &[u8], block: usize) -> __m128i {
        // SAFETY: the caller guarantees `block + 16 <= line.len()`.
        unsafe { _mm_loadu_si128(line.as_ptr().add(block) as *const __m128i) }
    }

    /// Packs the two rows' masked dwords: `_mm_packs_epi32` then
    /// `_mm_packus_epi16`; returns the two `uint32_t`s, `lo` from `line1` and
    /// `hi` from `line2`.
    #[inline]
    pub(super) unsafe fn pack_pair(a: __m128i, b: __m128i) -> (u32, u32) {
        // SAFETY: SSE2 integer ops only.
        unsafe {
            let packed = _mm_packs_epi32(a, b);
            let packed = _mm_packus_epi16(packed, packed);
            let lo = _mm_cvtsi128_si32(packed) as u32;
            let hi = _mm_cvtsi128_si32(_mm_srli_si128(packed, 4)) as u32;
            (lo, hi)
        }
    }

    /// The luma pack: mask Y (`0x0000FF00`), shift 1 byte.
    #[inline]
    pub(super) unsafe fn pack_lum(l1: __m128i, l2: __m128i) -> (u32, u32) {
        // SAFETY: SSE2 integer ops only.
        unsafe {
            let m = _mm_set1_epi32(0x0000_FF00);
            pack_pair(_mm_srli_si128(_mm_and_si128(l1, m), 1),
                      _mm_srli_si128(_mm_and_si128(l2, m), 1))
        }
    }

    /// The U pack with no shift (the C's `pack_val`).
    #[inline]
    pub(super) unsafe fn pack_u(l1: __m128i, l2: __m128i) -> (u32, u32) {
        // SAFETY: SSE2 integer ops only.
        unsafe {
            let m = _mm_set1_epi32(0x0000_00FF);
            pack_pair(_mm_and_si128(l1, m), _mm_and_si128(l2, m))
        }
    }

    /// The V pack: mask `0x00FF0000`, shift 2 bytes.
    #[inline]
    pub(super) unsafe fn pack_v(l1: __m128i, l2: __m128i) -> (u32, u32) {
        // SAFETY: SSE2 integer ops only.
        unsafe {
            let m = _mm_set1_epi32(0x00FF_0000);
            pack_pair(_mm_srli_si128(_mm_and_si128(l1, m), 2),
                      _mm_srli_si128(_mm_and_si128(l2, m), 2))
        }
    }

    /// Sums the two rows under the `0x00FF` mask and halves-adds the adjacent
    /// dword lanes, returning the 16-bit lanes `[u01, v01, u23, v23, ...]` as
    /// the C's `avg_val` before the final shuffle.
    #[inline]
    pub(super) unsafe fn chroma_sum(l1: __m128i, l2: __m128i) -> __m128i {
        // SAFETY: SSE2 integer ops only.
        unsafe {
            let uv_mask = _mm_set1_epi16(0x00FF);
            let add = _mm_add_epi64(_mm_and_si128(l1, uv_mask), _mm_and_si128(l2, uv_mask));
            let avg = _mm_add_epi64(add, _mm_shuffle_epi32(add, SHUFFLE_D1));
            _mm_srai_epi16(avg, 2)
        }
    }

    /// The NV12 chroma store: `[u01, v01, u23, v23]` packed.
    #[inline]
    pub(super) unsafe fn chroma_nv12(avg: __m128i) -> u32 {
        // SAFETY: SSE2 integer ops only.
        unsafe {
            let avg = _mm_shuffle_epi32(avg, SHUFFLE_D8);
            let avg = _mm_packus_epi16(avg, avg);
            _mm_cvtsi128_si32(avg) as u32
        }
    }

    /// The I420 chroma store: `[u01, u23 | v01, v23]` packed; U in the low
    /// half, V in the high half.
    #[inline]
    pub(super) unsafe fn chroma_i420(avg: __m128i) -> u32 {
        // SAFETY: SSE2 integer ops only.
        unsafe {
            let avg = _mm_shuffle_epi32(avg, SHUFFLE_D8);
            let avg = _mm_shufflelo_epi16(avg, SHUFFLE_D8);
            let avg = _mm_packus_epi16(avg, avg);
            _mm_cvtsi128_si32(avg) as u32
        }
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse2")]
unsafe fn block_i420_sse2(
    luma: &mut [u8],
    lum_stride: usize,
    u_row: &mut [u8],
    v_row: &mut [u8],
    line1: &[u8],
    line2: &[u8],
    block: usize,
    i: usize,
) {
    // SAFETY: every byte offset used here is inside the slices, which the
    // caller sizes for the block's bytes (`block + 16` for the rows,
    // `stride + bytes` for the luma pair, `blocks * 2` for the chroma rows).
    unsafe {
        let l1 = sse2::mask_loadu(line1, block);
        let l2 = sse2::mask_loadu(line2, block);
        let pixel = i * 4;
        let (lo, hi) = sse2::pack_lum(l1, l2);
        luma[pixel..pixel + 4].copy_from_slice(&lo.to_ne_bytes());
        luma[lum_stride + pixel..lum_stride + pixel + 4].copy_from_slice(&hi.to_ne_bytes());

        let avg = sse2::chroma_sum(l1, l2);
        let packed = sse2::chroma_i420(avg).to_ne_bytes();
        u_row[i * 2..i * 2 + 2].copy_from_slice(&packed[..2]);
        v_row[i * 2..i * 2 + 2].copy_from_slice(&packed[2..]);
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse2")]
unsafe fn block_nv12_sse2(
    luma: &mut [u8],
    lum_stride: usize,
    chroma_row: &mut [u8],
    line1: &[u8],
    line2: &[u8],
    block: usize,
    i: usize,
) {
    // SAFETY: as above.
    unsafe {
        let l1 = sse2::mask_loadu(line1, block);
        let l2 = sse2::mask_loadu(line2, block);
        let pixel = i * 4;
        let (lo, hi) = sse2::pack_lum(l1, l2);
        luma[pixel..pixel + 4].copy_from_slice(&lo.to_ne_bytes());
        luma[lum_stride + pixel..lum_stride + pixel + 4].copy_from_slice(&hi.to_ne_bytes());

        let avg = sse2::chroma_sum(l1, l2);
        let packed = sse2::chroma_nv12(avg).to_ne_bytes();
        chroma_row[pixel..pixel + 4].copy_from_slice(&packed);
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse2")]
unsafe fn block_i444_sse2(
    luma: &mut [u8],
    u_row: &mut [u8],
    v_row: &mut [u8],
    stride: usize,
    line1: &[u8],
    line2: &[u8],
    block: usize,
    i: usize,
) {
    // SAFETY: as above.
    unsafe {
        let l1 = sse2::mask_loadu(line1, block);
        let l2 = sse2::mask_loadu(line2, block);
        let pixel = i * 4;
        let (lum_lo, lum_hi) = sse2::pack_lum(l1, l2);
        luma[pixel..pixel + 4].copy_from_slice(&lum_lo.to_ne_bytes());
        luma[stride + pixel..stride + pixel + 4].copy_from_slice(&lum_hi.to_ne_bytes());

        let (u_lo, u_hi) = sse2::pack_u(l1, l2);
        u_row[pixel..pixel + 4].copy_from_slice(&u_lo.to_ne_bytes());
        u_row[stride + pixel..stride + pixel + 4].copy_from_slice(&u_hi.to_ne_bytes());

        let (v_lo, v_hi) = sse2::pack_v(l1, l2);
        v_row[pixel..pixel + 4].copy_from_slice(&v_lo.to_ne_bytes());
        v_row[stride + pixel..stride + pixel + 4].copy_from_slice(&v_hi.to_ne_bytes());
    }
}
