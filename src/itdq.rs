use super::def::*;
use super::hawktracer::*;
use super::tbl::*;
use super::tracer::*;
use super::util::*;

use std::ops::{Add, Sub};

const ITX_SHIFT1: usize = (7); /* shift after 1st IT stage */
const ITX_SHIFT2: usize = (12 - (BIT_DEPTH - 8)); /* shift after 2nd IT stage */

const MAX_TX_DYNAMIC_RANGE: i16 = 15;
const MAX_TX_VAL: i16 = 32767;
const MIN_TX_VAL: i16 = -32768;

const MAX_TX_DYNAMIC_RANGE_32: i32 = 31;
const MAX_TX_VAL_32: i32 = 2147483647;
const MIN_TX_VAL_32: i32 = -2147483648;

#[inline]
fn ITX_CLIP(x: i64) -> i16 {
    if x < MIN_TX_VAL as i64 {
        MIN_TX_VAL
    } else if x > MAX_TX_VAL as i64 {
        MAX_TX_VAL
    } else {
        x as i16
    }
}
#[inline]
fn ITX_CLIP_32(x: i64) -> i32 {
    if x <= MIN_TX_VAL_32 as i64 {
        MIN_TX_VAL_32
    } else if x >= MAX_TX_VAL_32 as i64 {
        MAX_TX_VAL_32
    } else {
        x as i32
    }
}

#[hawktracer(evc_sub_block_itdq)]
pub(crate) fn evc_sub_block_itdq(
    tracer: &mut Option<Tracer>,
    coef: &mut Vec<Vec<i16>>, //[[i16; MAX_CU_DIM]],
    log2_cuw: u8,
    log2_cuh: u8,
    qp_y: u8,
    qp_u: u8,
    qp_v: u8,
    flag: &[bool],
) {
    let qp: [u8; N_C] = [qp_y, qp_u, qp_v];
    let mut scale = 0i32;

    for c in 0..N_C {
        let chroma = if c > 0 { 1 } else { 0 };
        if flag[c] {
            scale = (evc_tbl_dq_scale_b[qp[c] as usize % 6] as i32) << (qp[c] / 6) as i32;

            evc_itdq(
                &mut coef[c],
                (log2_cuw - chroma) as usize,
                (log2_cuh - chroma) as usize,
                scale,
            );

            TRACE_RESI(
                tracer,
                c,
                1 << (log2_cuw - chroma) as usize,
                1 << (log2_cuh - chroma) as usize,
                &coef[c],
            );
        }
    }
}

#[hawktracer(evc_dquant)]
fn evc_dquant(coef: &mut [i16], log2_w: usize, log2_h: usize, scale: i32, offset: i32, shift: u8) {
    let ns_scale: i64 = if (log2_w + log2_h) & 1 != 0 { 181 } else { 1 };
    for i in 0..1 << (log2_w + log2_h) {
        let lev = (coef[i] as i64 * (scale as i64 * ns_scale) + offset as i64) >> shift as i64;
        coef[i] = EVC_CLIP3(-32768, 32767, lev) as i16;
    }
}

#[hawktracer(itx_pb2b)]
fn itx_pb2b(coef: &mut [i16], shift0: usize, shift1: usize, line: usize) {
    let mut tb = [0i32; 4];

    let add0 = if shift0 == 0 {
        0
    } else {
        1 << (shift0 - 1) as i64
    };
    for j in 0..line {
        /* E and O */
        let E = coef[0 * line + j] as i64 + coef[1 * line + j] as i64;
        let O = coef[0 * line + j] as i64 - coef[1 * line + j] as i64;

        tb[j * 2 + 0] = ITX_CLIP_32((evc_tbl_tm2[0][0] as i64 * E + add0) >> shift0 as i64);
        tb[j * 2 + 1] = ITX_CLIP_32((evc_tbl_tm2[1][0] as i64 * O + add0) >> shift0 as i64);
    }

    let add1 = if shift1 == 0 {
        0
    } else {
        1 << (shift1 - 1) as i64
    };
    for j in 0..line {
        /* E and O */
        let E = tb[0 * line + j] as i64 + tb[1 * line + j] as i64;
        let O = tb[0 * line + j] as i64 - tb[1 * line + j] as i64;

        coef[j * 2 + 0] = ITX_CLIP((evc_tbl_tm2[0][0] as i64 * E + add1) >> shift1 as i64);
        coef[j * 2 + 1] = ITX_CLIP((evc_tbl_tm2[1][0] as i64 * O + add1) >> shift1 as i64);
    }
}

#[hawktracer(itx_pb4b)]
fn itx_pb4b(coef: &mut [i16], shift0: usize, shift1: usize, line: usize) {
    let mut tb = [0i32; 16];

    let add0 = if shift0 == 0 {
        0
    } else {
        1 << (shift0 - 1) as i64
    };
    for j in 0..line {
        /* Utilizing symmetry properties to the maximum to minimize the number of multiplications */
        let O0 = evc_tbl_tm4[1][0] as i64 * coef[1 * line + j] as i64
            + evc_tbl_tm4[3][0] as i64 * coef[3 * line + j] as i64;
        let O1 = evc_tbl_tm4[1][1] as i64 * coef[1 * line + j] as i64
            + evc_tbl_tm4[3][1] as i64 * coef[3 * line + j] as i64;
        let E0 = evc_tbl_tm4[0][0] as i64 * coef[0 * line + j] as i64
            + evc_tbl_tm4[2][0] as i64 * coef[2 * line + j] as i64;
        let E1 = evc_tbl_tm4[0][1] as i64 * coef[0 * line + j] as i64
            + evc_tbl_tm4[2][1] as i64 * coef[2 * line + j] as i64;

        tb[j * 4 + 0] = ITX_CLIP_32((E0 + O0 + add0) >> shift0 as i64);
        tb[j * 4 + 1] = ITX_CLIP_32((E1 + O1 + add0) >> shift0 as i64);
        tb[j * 4 + 2] = ITX_CLIP_32((E1 - O1 + add0) >> shift0 as i64);
        tb[j * 4 + 3] = ITX_CLIP_32((E0 - O0 + add0) >> shift0 as i64);
    }

    let add1 = if shift1 == 0 {
        0
    } else {
        1 << (shift1 - 1) as i64
    };
    for j in 0..line {
        /* Utilizing symmetry properties to the maximum to minimize the number of multiplications */
        let O0 = evc_tbl_tm4[1][0] as i64 * tb[1 * line + j] as i64
            + evc_tbl_tm4[3][0] as i64 * tb[3 * line + j] as i64;
        let O1 = evc_tbl_tm4[1][1] as i64 * tb[1 * line + j] as i64
            + evc_tbl_tm4[3][1] as i64 * tb[3 * line + j] as i64;
        let E0 = evc_tbl_tm4[0][0] as i64 * tb[0 * line + j] as i64
            + evc_tbl_tm4[2][0] as i64 * tb[2 * line + j] as i64;
        let E1 = evc_tbl_tm4[0][1] as i64 * tb[0 * line + j] as i64
            + evc_tbl_tm4[2][1] as i64 * tb[2 * line + j] as i64;

        coef[j * 4 + 0] = ITX_CLIP((E0 + O0 + add1) >> shift1 as i64);
        coef[j * 4 + 1] = ITX_CLIP((E1 + O1 + add1) >> shift1 as i64);
        coef[j * 4 + 2] = ITX_CLIP((E1 - O1 + add1) >> shift1 as i64);
        coef[j * 4 + 3] = ITX_CLIP((E0 - O0 + add1) >> shift1 as i64);
    }
}

#[hawktracer(itx_pb8b)]
fn itx_pb8b(coef: &mut [i16], shift0: usize, shift1: usize, line: usize) {
    let mut tb = [0i32; 64];

    let mut E = [0i64; 4];
    let mut O = [0i64; 4];

    let add0 = if shift0 == 0 {
        0
    } else {
        1 << (shift0 - 1) as i64
    };
    for j in 0..line {
        /* Utilizing symmetry properties to the maximum to minimize the number of multiplications */
        for k in 0..4 {
            O[k] = evc_tbl_tm8[1][k] as i64 * coef[1 * line + j] as i64
                + evc_tbl_tm8[3][k] as i64 * coef[3 * line + j] as i64
                + evc_tbl_tm8[5][k] as i64 * coef[5 * line + j] as i64
                + evc_tbl_tm8[7][k] as i64 * coef[7 * line + j] as i64;
        }

        let EO0 = evc_tbl_tm8[2][0] as i64 * coef[2 * line + j] as i64
            + evc_tbl_tm8[6][0] as i64 * coef[6 * line + j] as i64;
        let EO1 = evc_tbl_tm8[2][1] as i64 * coef[2 * line + j] as i64
            + evc_tbl_tm8[6][1] as i64 * coef[6 * line + j] as i64;
        let EE0 = evc_tbl_tm8[0][0] as i64 * coef[0 * line + j] as i64
            + evc_tbl_tm8[4][0] as i64 * coef[4 * line + j] as i64;
        let EE1 = evc_tbl_tm8[0][1] as i64 * coef[0 * line + j] as i64
            + evc_tbl_tm8[4][1] as i64 * coef[4 * line + j] as i64;

        /* Combining even and odd terms at each hierarchy levels to calculate the final spatial domain vector */
        E[0] = EE0 + EO0;
        E[3] = EE0 - EO0;
        E[1] = EE1 + EO1;
        E[2] = EE1 - EO1;

        for k in 0..4 {
            tb[j * 8 + k] = ITX_CLIP_32((E[k] + O[k] + add0) >> shift0 as i64);
            tb[j * 8 + k + 4] = ITX_CLIP_32((E[3 - k] - O[3 - k] + add0) >> shift0 as i64);
        }
    }

    let add1 = if shift1 == 0 {
        0
    } else {
        1 << (shift1 - 1) as i64
    };
    for j in 0..line {
        /* Utilizing symmetry properties to the maximum to minimize the number of multiplications */
        for k in 0..4 {
            O[k] = evc_tbl_tm8[1][k] as i64 * tb[1 * line + j] as i64
                + evc_tbl_tm8[3][k] as i64 * tb[3 * line + j] as i64
                + evc_tbl_tm8[5][k] as i64 * tb[5 * line + j] as i64
                + evc_tbl_tm8[7][k] as i64 * tb[7 * line + j] as i64;
        }

        let EO0 = evc_tbl_tm8[2][0] as i64 * tb[2 * line + j] as i64
            + evc_tbl_tm8[6][0] as i64 * tb[6 * line + j] as i64;
        let EO1 = evc_tbl_tm8[2][1] as i64 * tb[2 * line + j] as i64
            + evc_tbl_tm8[6][1] as i64 * tb[6 * line + j] as i64;
        let EE0 = evc_tbl_tm8[0][0] as i64 * tb[0 * line + j] as i64
            + evc_tbl_tm8[4][0] as i64 * tb[4 * line + j] as i64;
        let EE1 = evc_tbl_tm8[0][1] as i64 * tb[0 * line + j] as i64
            + evc_tbl_tm8[4][1] as i64 * tb[4 * line + j] as i64;

        /* Combining even and odd terms at each hierarchy levels to calculate the final spatial domain vector */
        E[0] = EE0 + EO0;
        E[3] = EE0 - EO0;
        E[1] = EE1 + EO1;
        E[2] = EE1 - EO1;

        for k in 0..4 {
            coef[j * 8 + k] = ITX_CLIP((E[k] + O[k] + add1) >> shift1 as i64);
            coef[j * 8 + k + 4] = ITX_CLIP((E[3 - k] - O[3 - k] + add1) >> shift1 as i64);
        }
    }
}

#[hawktracer(itx_pb16b)]
fn itx_pb16b(coef: &mut [i16], shift0: usize, shift1: usize, line: usize) {
    let mut tb = [0i32; 256];

    let mut E = [0i64; 8];
    let mut O = [0i64; 8];
    let mut EE = [0i64; 4];
    let mut EO = [0i64; 4];
    let mut EEE = [0i64; 2];
    let mut EEO = [0i64; 2];

    let add0 = if shift0 == 0 {
        0
    } else {
        1 << (shift0 - 1) as i64
    };
    for j in 0..line {
        /* Utilizing symmetry properties to the maximum to minimize the number of multiplications */
        for k in 0..8 {
            O[k] = evc_tbl_tm16[1][k] as i64 * coef[1 * line + j] as i64
                + evc_tbl_tm16[3][k] as i64 * coef[3 * line + j] as i64
                + evc_tbl_tm16[5][k] as i64 * coef[5 * line + j] as i64
                + evc_tbl_tm16[7][k] as i64 * coef[7 * line + j] as i64
                + evc_tbl_tm16[9][k] as i64 * coef[9 * line + j] as i64
                + evc_tbl_tm16[11][k] as i64 * coef[11 * line + j] as i64
                + evc_tbl_tm16[13][k] as i64 * coef[13 * line + j] as i64
                + evc_tbl_tm16[15][k] as i64 * coef[15 * line + j] as i64;
        }

        for k in 0..4 {
            EO[k] = evc_tbl_tm16[2][k] as i64 * coef[2 * line + j] as i64
                + evc_tbl_tm16[6][k] as i64 * coef[6 * line + j] as i64
                + evc_tbl_tm16[10][k] as i64 * coef[10 * line + j] as i64
                + evc_tbl_tm16[14][k] as i64 * coef[14 * line + j] as i64;
        }

        EEO[0] = evc_tbl_tm16[4][0] as i64 * coef[4 * line + j] as i64
            + evc_tbl_tm16[12][0] as i64 * coef[12 * line + j] as i64;
        EEE[0] = evc_tbl_tm16[0][0] as i64 * coef[0 * line + j] as i64
            + evc_tbl_tm16[8][0] as i64 * coef[8 * line + j] as i64;
        EEO[1] = evc_tbl_tm16[4][1] as i64 * coef[4 * line + j] as i64
            + evc_tbl_tm16[12][1] as i64 * coef[12 * line + j] as i64;
        EEE[1] = evc_tbl_tm16[0][1] as i64 * coef[0 * line + j] as i64
            + evc_tbl_tm16[8][1] as i64 * coef[8 * line + j] as i64;

        /* Combining even and odd terms at each hierarchy levels to calculate the final spatial domain vector */
        for k in 0..2 {
            EE[k] = EEE[k] + EEO[k];
            EE[k + 2] = EEE[1 - k] - EEO[1 - k];
        }
        for k in 0..4 {
            E[k] = EE[k] + EO[k];
            E[k + 4] = EE[3 - k] - EO[3 - k];
        }

        for k in 0..8 {
            tb[j * 16 + k] = ITX_CLIP_32((E[k] + O[k] + add0) >> shift0 as i64);
            tb[j * 16 + k + 8] = ITX_CLIP_32((E[7 - k] - O[7 - k] + add0) >> shift0 as i64);
        }
    }

    let add1 = if shift1 == 0 {
        0
    } else {
        1 << (shift1 - 1) as i64
    };
    for j in 0..line {
        /* Utilizing symmetry properties to the maximum to minimize the number of multiplications */
        for k in 0..8 {
            O[k] = evc_tbl_tm16[1][k] as i64 * tb[1 * line + j] as i64
                + evc_tbl_tm16[3][k] as i64 * tb[3 * line + j] as i64
                + evc_tbl_tm16[5][k] as i64 * tb[5 * line + j] as i64
                + evc_tbl_tm16[7][k] as i64 * tb[7 * line + j] as i64
                + evc_tbl_tm16[9][k] as i64 * tb[9 * line + j] as i64
                + evc_tbl_tm16[11][k] as i64 * tb[11 * line + j] as i64
                + evc_tbl_tm16[13][k] as i64 * tb[13 * line + j] as i64
                + evc_tbl_tm16[15][k] as i64 * tb[15 * line + j] as i64;
        }

        for k in 0..4 {
            EO[k] = evc_tbl_tm16[2][k] as i64 * tb[2 * line + j] as i64
                + evc_tbl_tm16[6][k] as i64 * tb[6 * line + j] as i64
                + evc_tbl_tm16[10][k] as i64 * tb[10 * line + j] as i64
                + evc_tbl_tm16[14][k] as i64 * tb[14 * line + j] as i64;
        }

        EEO[0] = evc_tbl_tm16[4][0] as i64 * tb[4 * line + j] as i64
            + evc_tbl_tm16[12][0] as i64 * tb[12 * line + j] as i64;
        EEE[0] = evc_tbl_tm16[0][0] as i64 * tb[0 * line + j] as i64
            + evc_tbl_tm16[8][0] as i64 * tb[8 * line + j] as i64;
        EEO[1] = evc_tbl_tm16[4][1] as i64 * tb[4 * line + j] as i64
            + evc_tbl_tm16[12][1] as i64 * tb[12 * line + j] as i64;
        EEE[1] = evc_tbl_tm16[0][1] as i64 * tb[0 * line + j] as i64
            + evc_tbl_tm16[8][1] as i64 * tb[8 * line + j] as i64;

        /* Combining even and odd terms at each hierarchy levels to calculate the final spatial domain vector */
        for k in 0..2 {
            EE[k] = EEE[k] + EEO[k];
            EE[k + 2] = EEE[1 - k] - EEO[1 - k];
        }
        for k in 0..4 {
            E[k] = EE[k] + EO[k];
            E[k + 4] = EE[3 - k] - EO[3 - k];
        }

        for k in 0..8 {
            coef[j * 16 + k] = ITX_CLIP((E[k] + O[k] + add1) >> shift1 as i64);
            coef[j * 16 + k + 8] = ITX_CLIP((E[7 - k] - O[7 - k] + add1) >> shift1 as i64);
        }
    }
}

#[hawktracer(itx_pb32b)]
fn itx_pb32b(coef: &mut [i16], shift0: usize, shift1: usize, line: usize) {
    let mut tb = [0i32; 32 * 32];

    let mut E = [0i64; 16];
    let mut O = [0i64; 16];
    let mut EE = [0i64; 8];
    let mut EO = [0i64; 8];
    let mut EEE = [0i64; 4];
    let mut EEO = [0i64; 4];
    let mut EEEE = [0i64; 2];
    let mut EEEO = [0i64; 2];

    let add0 = if shift0 == 0 {
        0
    } else {
        1 << (shift0 - 1) as i64
    };
    for j in 0..line {
        for k in 0..16 {
            O[k] = evc_tbl_tm32[1][k] as i64 * coef[1 * line + j] as i64
                + evc_tbl_tm32[3][k] as i64 * coef[3 * line + j] as i64
                + evc_tbl_tm32[5][k] as i64 * coef[5 * line + j] as i64
                + evc_tbl_tm32[7][k] as i64 * coef[7 * line + j] as i64
                + evc_tbl_tm32[9][k] as i64 * coef[9 * line + j] as i64
                + evc_tbl_tm32[11][k] as i64 * coef[11 * line + j] as i64
                + evc_tbl_tm32[13][k] as i64 * coef[13 * line + j] as i64
                + evc_tbl_tm32[15][k] as i64 * coef[15 * line + j] as i64
                + evc_tbl_tm32[17][k] as i64 * coef[17 * line + j] as i64
                + evc_tbl_tm32[19][k] as i64 * coef[19 * line + j] as i64
                + evc_tbl_tm32[21][k] as i64 * coef[21 * line + j] as i64
                + evc_tbl_tm32[23][k] as i64 * coef[23 * line + j] as i64
                + evc_tbl_tm32[25][k] as i64 * coef[25 * line + j] as i64
                + evc_tbl_tm32[27][k] as i64 * coef[27 * line + j] as i64
                + evc_tbl_tm32[29][k] as i64 * coef[29 * line + j] as i64
                + evc_tbl_tm32[31][k] as i64 * coef[31 * line + j] as i64;
        }

        for k in 0..8 {
            EO[k] = evc_tbl_tm32[2][k] as i64 * coef[2 * line + j] as i64
                + evc_tbl_tm32[6][k] as i64 * coef[6 * line + j] as i64
                + evc_tbl_tm32[10][k] as i64 * coef[10 * line + j] as i64
                + evc_tbl_tm32[14][k] as i64 * coef[14 * line + j] as i64
                + evc_tbl_tm32[18][k] as i64 * coef[18 * line + j] as i64
                + evc_tbl_tm32[22][k] as i64 * coef[22 * line + j] as i64
                + evc_tbl_tm32[26][k] as i64 * coef[26 * line + j] as i64
                + evc_tbl_tm32[30][k] as i64 * coef[30 * line + j] as i64;
        }

        for k in 0..4 {
            EEO[k] = evc_tbl_tm32[4][k] as i64 * coef[4 * line + j] as i64
                + evc_tbl_tm32[12][k] as i64 * coef[12 * line + j] as i64
                + evc_tbl_tm32[20][k] as i64 * coef[20 * line + j] as i64
                + evc_tbl_tm32[28][k] as i64 * coef[28 * line + j] as i64;
        }

        EEEO[0] = evc_tbl_tm32[8][0] as i64 * coef[8 * line + j] as i64
            + evc_tbl_tm32[24][0] as i64 * coef[24 * line + j] as i64;
        EEEO[1] = evc_tbl_tm32[8][1] as i64 * coef[8 * line + j] as i64
            + evc_tbl_tm32[24][1] as i64 * coef[24 * line + j] as i64;
        EEEE[0] = evc_tbl_tm32[0][0] as i64 * coef[0 * line + j] as i64
            + evc_tbl_tm32[16][0] as i64 * coef[16 * line + j] as i64;
        EEEE[1] = evc_tbl_tm32[0][1] as i64 * coef[0 * line + j] as i64
            + evc_tbl_tm32[16][1] as i64 * coef[16 * line + j] as i64;

        EEE[0] = EEEE[0] + EEEO[0];
        EEE[3] = EEEE[0] - EEEO[0];
        EEE[1] = EEEE[1] + EEEO[1];
        EEE[2] = EEEE[1] - EEEO[1];
        for k in 0..4 {
            EE[k] = EEE[k] + EEO[k];
            EE[k + 4] = EEE[3 - k] - EEO[3 - k];
        }
        for k in 0..8 {
            E[k] = EE[k] + EO[k];
            E[k + 8] = EE[7 - k] - EO[7 - k];
        }

        for k in 0..16 {
            tb[j * 32 + k] = ITX_CLIP_32((E[k] + O[k] + add0) >> shift0 as i64);
            tb[j * 32 + k + 16] = ITX_CLIP_32((E[15 - k] - O[15 - k] + add0) >> shift0 as i64);
        }
    }

    let add1 = if shift1 == 0 {
        0
    } else {
        1 << (shift1 - 1) as i64
    };
    for j in 0..line {
        for k in 0..16 {
            O[k] = evc_tbl_tm32[1][k] as i64 * tb[1 * line + j] as i64
                + evc_tbl_tm32[3][k] as i64 * tb[3 * line + j] as i64
                + evc_tbl_tm32[5][k] as i64 * tb[5 * line + j] as i64
                + evc_tbl_tm32[7][k] as i64 * tb[7 * line + j] as i64
                + evc_tbl_tm32[9][k] as i64 * tb[9 * line + j] as i64
                + evc_tbl_tm32[11][k] as i64 * tb[11 * line + j] as i64
                + evc_tbl_tm32[13][k] as i64 * tb[13 * line + j] as i64
                + evc_tbl_tm32[15][k] as i64 * tb[15 * line + j] as i64
                + evc_tbl_tm32[17][k] as i64 * tb[17 * line + j] as i64
                + evc_tbl_tm32[19][k] as i64 * tb[19 * line + j] as i64
                + evc_tbl_tm32[21][k] as i64 * tb[21 * line + j] as i64
                + evc_tbl_tm32[23][k] as i64 * tb[23 * line + j] as i64
                + evc_tbl_tm32[25][k] as i64 * tb[25 * line + j] as i64
                + evc_tbl_tm32[27][k] as i64 * tb[27 * line + j] as i64
                + evc_tbl_tm32[29][k] as i64 * tb[29 * line + j] as i64
                + evc_tbl_tm32[31][k] as i64 * tb[31 * line + j] as i64;
        }

        for k in 0..8 {
            EO[k] = evc_tbl_tm32[2][k] as i64 * tb[2 * line + j] as i64
                + evc_tbl_tm32[6][k] as i64 * tb[6 * line + j] as i64
                + evc_tbl_tm32[10][k] as i64 * tb[10 * line + j] as i64
                + evc_tbl_tm32[14][k] as i64 * tb[14 * line + j] as i64
                + evc_tbl_tm32[18][k] as i64 * tb[18 * line + j] as i64
                + evc_tbl_tm32[22][k] as i64 * tb[22 * line + j] as i64
                + evc_tbl_tm32[26][k] as i64 * tb[26 * line + j] as i64
                + evc_tbl_tm32[30][k] as i64 * tb[30 * line + j] as i64;
        }

        for k in 0..4 {
            EEO[k] = evc_tbl_tm32[4][k] as i64 * tb[4 * line + j] as i64
                + evc_tbl_tm32[12][k] as i64 * tb[12 * line + j] as i64
                + evc_tbl_tm32[20][k] as i64 * tb[20 * line + j] as i64
                + evc_tbl_tm32[28][k] as i64 * tb[28 * line + j] as i64;
        }

        EEEO[0] = evc_tbl_tm32[8][0] as i64 * tb[8 * line + j] as i64
            + evc_tbl_tm32[24][0] as i64 * tb[24 * line + j] as i64;
        EEEO[1] = evc_tbl_tm32[8][1] as i64 * tb[8 * line + j] as i64
            + evc_tbl_tm32[24][1] as i64 * tb[24 * line + j] as i64;
        EEEE[0] = evc_tbl_tm32[0][0] as i64 * tb[0 * line + j] as i64
            + evc_tbl_tm32[16][0] as i64 * tb[16 * line + j] as i64;
        EEEE[1] = evc_tbl_tm32[0][1] as i64 * tb[0 * line + j] as i64
            + evc_tbl_tm32[16][1] as i64 * tb[16 * line + j] as i64;

        EEE[0] = EEEE[0] + EEEO[0];
        EEE[3] = EEEE[0] - EEEO[0];
        EEE[1] = EEEE[1] + EEEO[1];
        EEE[2] = EEEE[1] - EEEO[1];
        for k in 0..4 {
            EE[k] = EEE[k] + EEO[k];
            EE[k + 4] = EEE[3 - k] - EEO[3 - k];
        }
        for k in 0..8 {
            E[k] = EE[k] + EO[k];
            E[k + 8] = EE[7 - k] - EO[7 - k];
        }

        for k in 0..16 {
            coef[j * 32 + k] = ITX_CLIP((E[k] + O[k] + add1) >> shift1 as i64);
            coef[j * 32 + k + 16] = ITX_CLIP((E[15 - k] - O[15 - k] + add1) >> shift1 as i64);
        }
    }
}

#[hawktracer(itx_pb64b)]
fn itx_pb64b(coef: &mut [i16], shift0: usize, shift1: usize, line: usize) {
    let mut tb = [0i32; 64 * 64];

    let mut E = [0i64; 32];
    let mut O = [0i64; 32];
    let mut EE = [0i64; 16];
    let mut EO = [0i64; 16];
    let mut EEE = [0i64; 8];
    let mut EEO = [0i64; 8];
    let mut EEEE = [0i64; 4];
    let mut EEEO = [0i64; 4];
    let mut EEEEE = [0i64; 2];
    let mut EEEEO = [0i64; 2];

    let add0 = if shift0 == 0 {
        0
    } else {
        1 << (shift0 - 1) as i64
    };
    for j in 0..line {
        for k in 0..32 {
            O[k] = evc_tbl_tm64[1][k] as i64 * coef[1 * line + j] as i64
                + evc_tbl_tm64[3][k] as i64 * coef[3 * line + j] as i64
                + evc_tbl_tm64[5][k] as i64 * coef[5 * line + j] as i64
                + evc_tbl_tm64[7][k] as i64 * coef[7 * line + j] as i64
                + evc_tbl_tm64[9][k] as i64 * coef[9 * line + j] as i64
                + evc_tbl_tm64[11][k] as i64 * coef[11 * line + j] as i64
                + evc_tbl_tm64[13][k] as i64 * coef[13 * line + j] as i64
                + evc_tbl_tm64[15][k] as i64 * coef[15 * line + j] as i64
                + evc_tbl_tm64[17][k] as i64 * coef[17 * line + j] as i64
                + evc_tbl_tm64[19][k] as i64 * coef[19 * line + j] as i64
                + evc_tbl_tm64[21][k] as i64 * coef[21 * line + j] as i64
                + evc_tbl_tm64[23][k] as i64 * coef[23 * line + j] as i64
                + evc_tbl_tm64[25][k] as i64 * coef[25 * line + j] as i64
                + evc_tbl_tm64[27][k] as i64 * coef[27 * line + j] as i64
                + evc_tbl_tm64[29][k] as i64 * coef[29 * line + j] as i64
                + evc_tbl_tm64[31][k] as i64 * coef[31 * line + j] as i64
                + evc_tbl_tm64[33][k] as i64 * coef[33 * line + j] as i64
                + evc_tbl_tm64[35][k] as i64 * coef[35 * line + j] as i64
                + evc_tbl_tm64[37][k] as i64 * coef[37 * line + j] as i64
                + evc_tbl_tm64[39][k] as i64 * coef[39 * line + j] as i64
                + evc_tbl_tm64[41][k] as i64 * coef[41 * line + j] as i64
                + evc_tbl_tm64[43][k] as i64 * coef[43 * line + j] as i64
                + evc_tbl_tm64[45][k] as i64 * coef[45 * line + j] as i64
                + evc_tbl_tm64[47][k] as i64 * coef[47 * line + j] as i64
                + evc_tbl_tm64[49][k] as i64 * coef[49 * line + j] as i64
                + evc_tbl_tm64[51][k] as i64 * coef[51 * line + j] as i64
                + evc_tbl_tm64[53][k] as i64 * coef[53 * line + j] as i64
                + evc_tbl_tm64[55][k] as i64 * coef[55 * line + j] as i64
                + evc_tbl_tm64[57][k] as i64 * coef[57 * line + j] as i64
                + evc_tbl_tm64[59][k] as i64 * coef[59 * line + j] as i64
                + evc_tbl_tm64[61][k] as i64 * coef[61 * line + j] as i64
                + evc_tbl_tm64[63][k] as i64 * coef[63 * line + j] as i64;
        }

        for k in 0..16 {
            EO[k] = evc_tbl_tm64[2][k] as i64 * coef[2 * line + j] as i64
                + evc_tbl_tm64[6][k] as i64 * coef[6 * line + j] as i64
                + evc_tbl_tm64[10][k] as i64 * coef[10 * line + j] as i64
                + evc_tbl_tm64[14][k] as i64 * coef[14 * line + j] as i64
                + evc_tbl_tm64[18][k] as i64 * coef[18 * line + j] as i64
                + evc_tbl_tm64[22][k] as i64 * coef[22 * line + j] as i64
                + evc_tbl_tm64[26][k] as i64 * coef[26 * line + j] as i64
                + evc_tbl_tm64[30][k] as i64 * coef[30 * line + j] as i64
                + evc_tbl_tm64[34][k] as i64 * coef[34 * line + j] as i64
                + evc_tbl_tm64[38][k] as i64 * coef[38 * line + j] as i64
                + evc_tbl_tm64[42][k] as i64 * coef[42 * line + j] as i64
                + evc_tbl_tm64[46][k] as i64 * coef[46 * line + j] as i64
                + evc_tbl_tm64[50][k] as i64 * coef[50 * line + j] as i64
                + evc_tbl_tm64[54][k] as i64 * coef[54 * line + j] as i64
                + evc_tbl_tm64[58][k] as i64 * coef[58 * line + j] as i64
                + evc_tbl_tm64[62][k] as i64 * coef[62 * line + j] as i64;
        }

        for k in 0..8 {
            EEO[k] = evc_tbl_tm64[4][k] as i64 * coef[4 * line + j] as i64
                + evc_tbl_tm64[12][k] as i64 * coef[12 * line + j] as i64
                + evc_tbl_tm64[20][k] as i64 * coef[20 * line + j] as i64
                + evc_tbl_tm64[28][k] as i64 * coef[28 * line + j] as i64
                + evc_tbl_tm64[36][k] as i64 * coef[36 * line + j] as i64
                + evc_tbl_tm64[44][k] as i64 * coef[44 * line + j] as i64
                + evc_tbl_tm64[52][k] as i64 * coef[52 * line + j] as i64
                + evc_tbl_tm64[60][k] as i64 * coef[60 * line + j] as i64;
        }

        for k in 0..4 {
            EEEO[k] = evc_tbl_tm64[8][k] as i64 * coef[8 * line + j] as i64
                + evc_tbl_tm64[24][k] as i64 * coef[24 * line + j] as i64
                + evc_tbl_tm64[40][k] as i64 * coef[40 * line + j] as i64
                + evc_tbl_tm64[56][k] as i64 * coef[56 * line + j] as i64;
        }
        EEEEO[0] = evc_tbl_tm64[16][0] as i64 * coef[16 * line + j] as i64
            + evc_tbl_tm64[48][0] as i64 * coef[48 * line + j] as i64;
        EEEEO[1] = evc_tbl_tm64[16][1] as i64 * coef[16 * line + j] as i64
            + evc_tbl_tm64[48][1] as i64 * coef[48 * line + j] as i64;
        EEEEE[0] = evc_tbl_tm64[0][0] as i64 * coef[0 * line + j] as i64
            + evc_tbl_tm64[32][0] as i64 * coef[32 * line + j] as i64;
        EEEEE[1] = evc_tbl_tm64[0][1] as i64 * coef[0 * line + j] as i64
            + evc_tbl_tm64[32][1] as i64 * coef[32 * line + j] as i64;

        for k in 0..2 {
            EEEE[k] = EEEEE[k] + EEEEO[k];
            EEEE[k + 2] = EEEEE[1 - k] - EEEEO[1 - k];
        }
        for k in 0..4 {
            EEE[k] = EEEE[k] + EEEO[k];
            EEE[k + 4] = EEEE[3 - k] - EEEO[3 - k];
        }
        for k in 0..8 {
            EE[k] = EEE[k] + EEO[k];
            EE[k + 8] = EEE[7 - k] - EEO[7 - k];
        }
        for k in 0..16 {
            E[k] = EE[k] + EO[k];
            E[k + 16] = EE[15 - k] - EO[15 - k];
        }

        for k in 0..32 {
            tb[j * 64 + k] = ITX_CLIP_32((E[k] + O[k] + add0) >> shift0 as i64);
            tb[j * 64 + k + 32] = ITX_CLIP_32((E[31 - k] - O[31 - k] + add0) >> shift0 as i64);
        }
    }

    let add1 = if shift1 == 0 {
        0
    } else {
        1 << (shift1 - 1) as i64
    };
    for j in 0..line {
        for k in 0..32 {
            O[k] = evc_tbl_tm64[1][k] as i64 * tb[1 * line + j] as i64
                + evc_tbl_tm64[3][k] as i64 * tb[3 * line + j] as i64
                + evc_tbl_tm64[5][k] as i64 * tb[5 * line + j] as i64
                + evc_tbl_tm64[7][k] as i64 * tb[7 * line + j] as i64
                + evc_tbl_tm64[9][k] as i64 * tb[9 * line + j] as i64
                + evc_tbl_tm64[11][k] as i64 * tb[11 * line + j] as i64
                + evc_tbl_tm64[13][k] as i64 * tb[13 * line + j] as i64
                + evc_tbl_tm64[15][k] as i64 * tb[15 * line + j] as i64
                + evc_tbl_tm64[17][k] as i64 * tb[17 * line + j] as i64
                + evc_tbl_tm64[19][k] as i64 * tb[19 * line + j] as i64
                + evc_tbl_tm64[21][k] as i64 * tb[21 * line + j] as i64
                + evc_tbl_tm64[23][k] as i64 * tb[23 * line + j] as i64
                + evc_tbl_tm64[25][k] as i64 * tb[25 * line + j] as i64
                + evc_tbl_tm64[27][k] as i64 * tb[27 * line + j] as i64
                + evc_tbl_tm64[29][k] as i64 * tb[29 * line + j] as i64
                + evc_tbl_tm64[31][k] as i64 * tb[31 * line + j] as i64
                + evc_tbl_tm64[33][k] as i64 * tb[33 * line + j] as i64
                + evc_tbl_tm64[35][k] as i64 * tb[35 * line + j] as i64
                + evc_tbl_tm64[37][k] as i64 * tb[37 * line + j] as i64
                + evc_tbl_tm64[39][k] as i64 * tb[39 * line + j] as i64
                + evc_tbl_tm64[41][k] as i64 * tb[41 * line + j] as i64
                + evc_tbl_tm64[43][k] as i64 * tb[43 * line + j] as i64
                + evc_tbl_tm64[45][k] as i64 * tb[45 * line + j] as i64
                + evc_tbl_tm64[47][k] as i64 * tb[47 * line + j] as i64
                + evc_tbl_tm64[49][k] as i64 * tb[49 * line + j] as i64
                + evc_tbl_tm64[51][k] as i64 * tb[51 * line + j] as i64
                + evc_tbl_tm64[53][k] as i64 * tb[53 * line + j] as i64
                + evc_tbl_tm64[55][k] as i64 * tb[55 * line + j] as i64
                + evc_tbl_tm64[57][k] as i64 * tb[57 * line + j] as i64
                + evc_tbl_tm64[59][k] as i64 * tb[59 * line + j] as i64
                + evc_tbl_tm64[61][k] as i64 * tb[61 * line + j] as i64
                + evc_tbl_tm64[63][k] as i64 * tb[63 * line + j] as i64;
        }

        for k in 0..16 {
            EO[k] = evc_tbl_tm64[2][k] as i64 * tb[2 * line + j] as i64
                + evc_tbl_tm64[6][k] as i64 * tb[6 * line + j] as i64
                + evc_tbl_tm64[10][k] as i64 * tb[10 * line + j] as i64
                + evc_tbl_tm64[14][k] as i64 * tb[14 * line + j] as i64
                + evc_tbl_tm64[18][k] as i64 * tb[18 * line + j] as i64
                + evc_tbl_tm64[22][k] as i64 * tb[22 * line + j] as i64
                + evc_tbl_tm64[26][k] as i64 * tb[26 * line + j] as i64
                + evc_tbl_tm64[30][k] as i64 * tb[30 * line + j] as i64
                + evc_tbl_tm64[34][k] as i64 * tb[34 * line + j] as i64
                + evc_tbl_tm64[38][k] as i64 * tb[38 * line + j] as i64
                + evc_tbl_tm64[42][k] as i64 * tb[42 * line + j] as i64
                + evc_tbl_tm64[46][k] as i64 * tb[46 * line + j] as i64
                + evc_tbl_tm64[50][k] as i64 * tb[50 * line + j] as i64
                + evc_tbl_tm64[54][k] as i64 * tb[54 * line + j] as i64
                + evc_tbl_tm64[58][k] as i64 * tb[58 * line + j] as i64
                + evc_tbl_tm64[62][k] as i64 * tb[62 * line + j] as i64;
        }

        for k in 0..8 {
            EEO[k] = evc_tbl_tm64[4][k] as i64 * tb[4 * line + j] as i64
                + evc_tbl_tm64[12][k] as i64 * tb[12 * line + j] as i64
                + evc_tbl_tm64[20][k] as i64 * tb[20 * line + j] as i64
                + evc_tbl_tm64[28][k] as i64 * tb[28 * line + j] as i64
                + evc_tbl_tm64[36][k] as i64 * tb[36 * line + j] as i64
                + evc_tbl_tm64[44][k] as i64 * tb[44 * line + j] as i64
                + evc_tbl_tm64[52][k] as i64 * tb[52 * line + j] as i64
                + evc_tbl_tm64[60][k] as i64 * tb[60 * line + j] as i64;
        }

        for k in 0..4 {
            EEEO[k] = evc_tbl_tm64[8][k] as i64 * tb[8 * line + j] as i64
                + evc_tbl_tm64[24][k] as i64 * tb[24 * line + j] as i64
                + evc_tbl_tm64[40][k] as i64 * tb[40 * line + j] as i64
                + evc_tbl_tm64[56][k] as i64 * tb[56 * line + j] as i64;
        }
        EEEEO[0] = evc_tbl_tm64[16][0] as i64 * tb[16 * line + j] as i64
            + evc_tbl_tm64[48][0] as i64 * tb[48 * line + j] as i64;
        EEEEO[1] = evc_tbl_tm64[16][1] as i64 * tb[16 * line + j] as i64
            + evc_tbl_tm64[48][1] as i64 * tb[48 * line + j] as i64;
        EEEEE[0] = evc_tbl_tm64[0][0] as i64 * tb[0 * line + j] as i64
            + evc_tbl_tm64[32][0] as i64 * tb[32 * line + j] as i64;
        EEEEE[1] = evc_tbl_tm64[0][1] as i64 * tb[0 * line + j] as i64
            + evc_tbl_tm64[32][1] as i64 * tb[32 * line + j] as i64;

        for k in 0..2 {
            EEEE[k] = EEEEE[k] + EEEEO[k];
            EEEE[k + 2] = EEEEE[1 - k] - EEEEO[1 - k];
        }
        for k in 0..4 {
            EEE[k] = EEEE[k] + EEEO[k];
            EEE[k + 4] = EEEE[3 - k] - EEEO[3 - k];
        }
        for k in 0..8 {
            EE[k] = EEE[k] + EEO[k];
            EE[k + 8] = EEE[7 - k] - EEO[7 - k];
        }
        for k in 0..16 {
            E[k] = EE[k] + EO[k];
            E[k + 16] = EE[15 - k] - EO[15 - k];
        }

        for k in 0..32 {
            coef[j * 64 + k] = ITX_CLIP((E[k] + O[k] + add1) >> shift1 as i64);
            coef[j * 64 + k + 32] = ITX_CLIP((E[31 - k] - O[31 - k] + add1) >> shift1 as i64);
        }
    }
}

type EVC_ITXB = fn(coef: &mut [i16], shift0: usize, shift1: usize, line: usize);

static tbl_itxb: [EVC_ITXB; MAX_TR_LOG2] = [
    itx_pb2b, itx_pb4b, itx_pb8b, itx_pb16b, itx_pb32b, itx_pb64b,
];

#[hawktracer(itx_pb2b0)]
fn itx_pb2b0(src: &[i16], dst: &mut [i32], shift: usize, line: usize) {
    let add = if shift == 0 {
        0
    } else {
        1 << (shift - 1) as i64
    };
    for j in 0..line {
        /* E and O */
        let E = src[0 * line + j] as i64 + src[1 * line + j] as i64;
        let O = src[0 * line + j] as i64 - src[1 * line + j] as i64;

        dst[j * 2 + 0] = ITX_CLIP_32((evc_tbl_tm2[0][0] as i64 * E + add) >> shift as i64);
        dst[j * 2 + 1] = ITX_CLIP_32((evc_tbl_tm2[1][0] as i64 * O + add) >> shift as i64);
    }
}

#[hawktracer(itx_pb4b0)]
fn itx_pb4b0(src: &[i16], dst: &mut [i32], shift: usize, line: usize) {
    let add = if shift == 0 {
        0
    } else {
        1 << (shift - 1) as i64
    };
    for j in 0..line {
        /* Utilizing symmetry properties to the maximum to minimize the number of multiplications */
        let O0 = evc_tbl_tm4[1][0] as i64 * src[1 * line + j] as i64
            + evc_tbl_tm4[3][0] as i64 * src[3 * line + j] as i64;
        let O1 = evc_tbl_tm4[1][1] as i64 * src[1 * line + j] as i64
            + evc_tbl_tm4[3][1] as i64 * src[3 * line + j] as i64;
        let E0 = evc_tbl_tm4[0][0] as i64 * src[0 * line + j] as i64
            + evc_tbl_tm4[2][0] as i64 * src[2 * line + j] as i64;
        let E1 = evc_tbl_tm4[0][1] as i64 * src[0 * line + j] as i64
            + evc_tbl_tm4[2][1] as i64 * src[2 * line + j] as i64;

        dst[j * 4 + 0] = ITX_CLIP_32((E0 + O0 + add) >> shift as i64);
        dst[j * 4 + 1] = ITX_CLIP_32((E1 + O1 + add) >> shift as i64);
        dst[j * 4 + 2] = ITX_CLIP_32((E1 - O1 + add) >> shift as i64);
        dst[j * 4 + 3] = ITX_CLIP_32((E0 - O0 + add) >> shift as i64);
    }
}

#[hawktracer(itx_pb8b0)]
fn itx_pb8b0(src: &[i16], dst: &mut [i32], shift: usize, line: usize) {
    let add = if shift == 0 {
        0
    } else {
        1 << (shift - 1) as i64
    };

    let mut E = [0i64; 4];
    let mut O = [0i64; 4];
    for j in 0..line {
        /* Utilizing symmetry properties to the maximum to minimize the number of multiplications */
        for k in 0..4 {
            O[k] = evc_tbl_tm8[1][k] as i64 * src[1 * line + j] as i64
                + evc_tbl_tm8[3][k] as i64 * src[3 * line + j] as i64
                + evc_tbl_tm8[5][k] as i64 * src[5 * line + j] as i64
                + evc_tbl_tm8[7][k] as i64 * src[7 * line + j] as i64;
        }

        let EO0 = evc_tbl_tm8[2][0] as i64 * src[2 * line + j] as i64
            + evc_tbl_tm8[6][0] as i64 * src[6 * line + j] as i64;
        let EO1 = evc_tbl_tm8[2][1] as i64 * src[2 * line + j] as i64
            + evc_tbl_tm8[6][1] as i64 * src[6 * line + j] as i64;
        let EE0 = evc_tbl_tm8[0][0] as i64 * src[0 * line + j] as i64
            + evc_tbl_tm8[4][0] as i64 * src[4 * line + j] as i64;
        let EE1 = evc_tbl_tm8[0][1] as i64 * src[0 * line + j] as i64
            + evc_tbl_tm8[4][1] as i64 * src[4 * line + j] as i64;

        /* Combining even and odd terms at each hierarchy levels to calculate the final spatial domain vector */
        E[0] = EE0 + EO0;
        E[3] = EE0 - EO0;
        E[1] = EE1 + EO1;
        E[2] = EE1 - EO1;

        for k in 0..4 {
            dst[j * 8 + k] = ITX_CLIP_32((E[k] + O[k] + add) >> shift as i64);
            dst[j * 8 + k + 4] = ITX_CLIP_32((E[3 - k] - O[3 - k] + add) >> shift as i64);
        }
    }
}

#[hawktracer(itx_pb16b0)]
fn itx_pb16b0(src: &[i16], dst: &mut [i32], shift: usize, line: usize) {
    let add = if shift == 0 {
        0
    } else {
        1 << (shift - 1) as i64
    };

    let mut E = [0i64; 8];
    let mut O = [0i64; 8];
    let mut EE = [0i64; 4];
    let mut EO = [0i64; 4];
    let mut EEE = [0i64; 2];
    let mut EEO = [0i64; 2];
    for j in 0..line {
        /* Utilizing symmetry properties to the maximum to minimize the number of multiplications */
        for k in 0..8 {
            O[k] = evc_tbl_tm16[1][k] as i64 * src[1 * line + j] as i64
                + evc_tbl_tm16[3][k] as i64 * src[3 * line + j] as i64
                + evc_tbl_tm16[5][k] as i64 * src[5 * line + j] as i64
                + evc_tbl_tm16[7][k] as i64 * src[7 * line + j] as i64
                + evc_tbl_tm16[9][k] as i64 * src[9 * line + j] as i64
                + evc_tbl_tm16[11][k] as i64 * src[11 * line + j] as i64
                + evc_tbl_tm16[13][k] as i64 * src[13 * line + j] as i64
                + evc_tbl_tm16[15][k] as i64 * src[15 * line + j] as i64;
        }

        for k in 0..4 {
            EO[k] = evc_tbl_tm16[2][k] as i64 * src[2 * line + j] as i64
                + evc_tbl_tm16[6][k] as i64 * src[6 * line + j] as i64
                + evc_tbl_tm16[10][k] as i64 * src[10 * line + j] as i64
                + evc_tbl_tm16[14][k] as i64 * src[14 * line + j] as i64;
        }

        EEO[0] = evc_tbl_tm16[4][0] as i64 * src[4 * line + j] as i64
            + evc_tbl_tm16[12][0] as i64 * src[12 * line + j] as i64;
        EEE[0] = evc_tbl_tm16[0][0] as i64 * src[0 * line + j] as i64
            + evc_tbl_tm16[8][0] as i64 * src[8 * line + j] as i64;
        EEO[1] = evc_tbl_tm16[4][1] as i64 * src[4 * line + j] as i64
            + evc_tbl_tm16[12][1] as i64 * src[12 * line + j] as i64;
        EEE[1] = evc_tbl_tm16[0][1] as i64 * src[0 * line + j] as i64
            + evc_tbl_tm16[8][1] as i64 * src[8 * line + j] as i64;

        /* Combining even and odd terms at each hierarchy levels to calculate the final spatial domain vector */
        for k in 0..2 {
            EE[k] = EEE[k] + EEO[k];
            EE[k + 2] = EEE[1 - k] - EEO[1 - k];
        }
        for k in 0..4 {
            E[k] = EE[k] + EO[k];
            E[k + 4] = EE[3 - k] - EO[3 - k];
        }

        for k in 0..8 {
            dst[j * 16 + k] = ITX_CLIP_32((E[k] + O[k] + add) >> shift as i64);
            dst[j * 16 + k + 8] = ITX_CLIP_32((E[7 - k] - O[7 - k] + add) >> shift as i64);
        }
    }
}

#[hawktracer(itx_pb32b0)]
fn itx_pb32b0(src: &[i16], dst: &mut [i32], shift: usize, line: usize) {
    let add = if shift == 0 {
        0
    } else {
        1 << (shift - 1) as i64
    };

    let mut E = [0i64; 16];
    let mut O = [0i64; 16];
    let mut EE = [0i64; 8];
    let mut EO = [0i64; 8];
    let mut EEE = [0i64; 4];
    let mut EEO = [0i64; 4];
    let mut EEEE = [0i64; 2];
    let mut EEEO = [0i64; 2];
    for j in 0..line {
        for k in 0..16 {
            O[k] = evc_tbl_tm32[1][k] as i64 * src[1 * line + j] as i64
                + evc_tbl_tm32[3][k] as i64 * src[3 * line + j] as i64
                + evc_tbl_tm32[5][k] as i64 * src[5 * line + j] as i64
                + evc_tbl_tm32[7][k] as i64 * src[7 * line + j] as i64
                + evc_tbl_tm32[9][k] as i64 * src[9 * line + j] as i64
                + evc_tbl_tm32[11][k] as i64 * src[11 * line + j] as i64
                + evc_tbl_tm32[13][k] as i64 * src[13 * line + j] as i64
                + evc_tbl_tm32[15][k] as i64 * src[15 * line + j] as i64
                + evc_tbl_tm32[17][k] as i64 * src[17 * line + j] as i64
                + evc_tbl_tm32[19][k] as i64 * src[19 * line + j] as i64
                + evc_tbl_tm32[21][k] as i64 * src[21 * line + j] as i64
                + evc_tbl_tm32[23][k] as i64 * src[23 * line + j] as i64
                + evc_tbl_tm32[25][k] as i64 * src[25 * line + j] as i64
                + evc_tbl_tm32[27][k] as i64 * src[27 * line + j] as i64
                + evc_tbl_tm32[29][k] as i64 * src[29 * line + j] as i64
                + evc_tbl_tm32[31][k] as i64 * src[31 * line + j] as i64;
        }

        for k in 0..8 {
            EO[k] = evc_tbl_tm32[2][k] as i64 * src[2 * line + j] as i64
                + evc_tbl_tm32[6][k] as i64 * src[6 * line + j] as i64
                + evc_tbl_tm32[10][k] as i64 * src[10 * line + j] as i64
                + evc_tbl_tm32[14][k] as i64 * src[14 * line + j] as i64
                + evc_tbl_tm32[18][k] as i64 * src[18 * line + j] as i64
                + evc_tbl_tm32[22][k] as i64 * src[22 * line + j] as i64
                + evc_tbl_tm32[26][k] as i64 * src[26 * line + j] as i64
                + evc_tbl_tm32[30][k] as i64 * src[30 * line + j] as i64;
        }

        for k in 0..4 {
            EEO[k] = evc_tbl_tm32[4][k] as i64 * src[4 * line + j] as i64
                + evc_tbl_tm32[12][k] as i64 * src[12 * line + j] as i64
                + evc_tbl_tm32[20][k] as i64 * src[20 * line + j] as i64
                + evc_tbl_tm32[28][k] as i64 * src[28 * line + j] as i64;
        }

        EEEO[0] = evc_tbl_tm32[8][0] as i64 * src[8 * line + j] as i64
            + evc_tbl_tm32[24][0] as i64 * src[24 * line + j] as i64;
        EEEO[1] = evc_tbl_tm32[8][1] as i64 * src[8 * line + j] as i64
            + evc_tbl_tm32[24][1] as i64 * src[24 * line + j] as i64;
        EEEE[0] = evc_tbl_tm32[0][0] as i64 * src[0 * line + j] as i64
            + evc_tbl_tm32[16][0] as i64 * src[16 * line + j] as i64;
        EEEE[1] = evc_tbl_tm32[0][1] as i64 * src[0 * line + j] as i64
            + evc_tbl_tm32[16][1] as i64 * src[16 * line + j] as i64;

        EEE[0] = EEEE[0] + EEEO[0];
        EEE[3] = EEEE[0] - EEEO[0];
        EEE[1] = EEEE[1] + EEEO[1];
        EEE[2] = EEEE[1] - EEEO[1];
        for k in 0..4 {
            EE[k] = EEE[k] + EEO[k];
            EE[k + 4] = EEE[3 - k] - EEO[3 - k];
        }
        for k in 0..8 {
            E[k] = EE[k] + EO[k];
            E[k + 8] = EE[7 - k] - EO[7 - k];
        }

        for k in 0..16 {
            dst[j * 32 + k] = ITX_CLIP_32((E[k] + O[k] + add) >> shift as i64);
            dst[j * 32 + k + 16] = ITX_CLIP_32((E[15 - k] - O[15 - k] + add) >> shift as i64);
        }
    }
}

#[hawktracer(itx_pb64b0)]
fn itx_pb64b0(src: &[i16], dst: &mut [i32], shift: usize, line: usize) {
    let add = if shift == 0 {
        0
    } else {
        1 << (shift - 1) as i64
    };
    let mut E = [0i64; 32];
    let mut O = [0i64; 32];
    let mut EE = [0i64; 16];
    let mut EO = [0i64; 16];
    let mut EEE = [0i64; 8];
    let mut EEO = [0i64; 8];
    let mut EEEE = [0i64; 4];
    let mut EEEO = [0i64; 4];
    let mut EEEEE = [0i64; 2];
    let mut EEEEO = [0i64; 2];
    for j in 0..line {
        for k in 0..32 {
            O[k] = evc_tbl_tm64[1][k] as i64 * src[1 * line + j] as i64
                + evc_tbl_tm64[3][k] as i64 * src[3 * line + j] as i64
                + evc_tbl_tm64[5][k] as i64 * src[5 * line + j] as i64
                + evc_tbl_tm64[7][k] as i64 * src[7 * line + j] as i64
                + evc_tbl_tm64[9][k] as i64 * src[9 * line + j] as i64
                + evc_tbl_tm64[11][k] as i64 * src[11 * line + j] as i64
                + evc_tbl_tm64[13][k] as i64 * src[13 * line + j] as i64
                + evc_tbl_tm64[15][k] as i64 * src[15 * line + j] as i64
                + evc_tbl_tm64[17][k] as i64 * src[17 * line + j] as i64
                + evc_tbl_tm64[19][k] as i64 * src[19 * line + j] as i64
                + evc_tbl_tm64[21][k] as i64 * src[21 * line + j] as i64
                + evc_tbl_tm64[23][k] as i64 * src[23 * line + j] as i64
                + evc_tbl_tm64[25][k] as i64 * src[25 * line + j] as i64
                + evc_tbl_tm64[27][k] as i64 * src[27 * line + j] as i64
                + evc_tbl_tm64[29][k] as i64 * src[29 * line + j] as i64
                + evc_tbl_tm64[31][k] as i64 * src[31 * line + j] as i64
                + evc_tbl_tm64[33][k] as i64 * src[33 * line + j] as i64
                + evc_tbl_tm64[35][k] as i64 * src[35 * line + j] as i64
                + evc_tbl_tm64[37][k] as i64 * src[37 * line + j] as i64
                + evc_tbl_tm64[39][k] as i64 * src[39 * line + j] as i64
                + evc_tbl_tm64[41][k] as i64 * src[41 * line + j] as i64
                + evc_tbl_tm64[43][k] as i64 * src[43 * line + j] as i64
                + evc_tbl_tm64[45][k] as i64 * src[45 * line + j] as i64
                + evc_tbl_tm64[47][k] as i64 * src[47 * line + j] as i64
                + evc_tbl_tm64[49][k] as i64 * src[49 * line + j] as i64
                + evc_tbl_tm64[51][k] as i64 * src[51 * line + j] as i64
                + evc_tbl_tm64[53][k] as i64 * src[53 * line + j] as i64
                + evc_tbl_tm64[55][k] as i64 * src[55 * line + j] as i64
                + evc_tbl_tm64[57][k] as i64 * src[57 * line + j] as i64
                + evc_tbl_tm64[59][k] as i64 * src[59 * line + j] as i64
                + evc_tbl_tm64[61][k] as i64 * src[61 * line + j] as i64
                + evc_tbl_tm64[63][k] as i64 * src[63 * line + j] as i64;
        }

        for k in 0..16 {
            EO[k] = evc_tbl_tm64[2][k] as i64 * src[2 * line + j] as i64
                + evc_tbl_tm64[6][k] as i64 * src[6 * line + j] as i64
                + evc_tbl_tm64[10][k] as i64 * src[10 * line + j] as i64
                + evc_tbl_tm64[14][k] as i64 * src[14 * line + j] as i64
                + evc_tbl_tm64[18][k] as i64 * src[18 * line + j] as i64
                + evc_tbl_tm64[22][k] as i64 * src[22 * line + j] as i64
                + evc_tbl_tm64[26][k] as i64 * src[26 * line + j] as i64
                + evc_tbl_tm64[30][k] as i64 * src[30 * line + j] as i64
                + evc_tbl_tm64[34][k] as i64 * src[34 * line + j] as i64
                + evc_tbl_tm64[38][k] as i64 * src[38 * line + j] as i64
                + evc_tbl_tm64[42][k] as i64 * src[42 * line + j] as i64
                + evc_tbl_tm64[46][k] as i64 * src[46 * line + j] as i64
                + evc_tbl_tm64[50][k] as i64 * src[50 * line + j] as i64
                + evc_tbl_tm64[54][k] as i64 * src[54 * line + j] as i64
                + evc_tbl_tm64[58][k] as i64 * src[58 * line + j] as i64
                + evc_tbl_tm64[62][k] as i64 * src[62 * line + j] as i64;
        }

        for k in 0..8 {
            EEO[k] = evc_tbl_tm64[4][k] as i64 * src[4 * line + j] as i64
                + evc_tbl_tm64[12][k] as i64 * src[12 * line + j] as i64
                + evc_tbl_tm64[20][k] as i64 * src[20 * line + j] as i64
                + evc_tbl_tm64[28][k] as i64 * src[28 * line + j] as i64
                + evc_tbl_tm64[36][k] as i64 * src[36 * line + j] as i64
                + evc_tbl_tm64[44][k] as i64 * src[44 * line + j] as i64
                + evc_tbl_tm64[52][k] as i64 * src[52 * line + j] as i64
                + evc_tbl_tm64[60][k] as i64 * src[60 * line + j] as i64;
        }

        for k in 0..4 {
            EEEO[k] = evc_tbl_tm64[8][k] as i64 * src[8 * line + j] as i64
                + evc_tbl_tm64[24][k] as i64 * src[24 * line + j] as i64
                + evc_tbl_tm64[40][k] as i64 * src[40 * line + j] as i64
                + evc_tbl_tm64[56][k] as i64 * src[56 * line + j] as i64;
        }
        EEEEO[0] = evc_tbl_tm64[16][0] as i64 * src[16 * line + j] as i64
            + evc_tbl_tm64[48][0] as i64 * src[48 * line + j] as i64;
        EEEEO[1] = evc_tbl_tm64[16][1] as i64 * src[16 * line + j] as i64
            + evc_tbl_tm64[48][1] as i64 * src[48 * line + j] as i64;
        EEEEE[0] = evc_tbl_tm64[0][0] as i64 * src[0 * line + j] as i64
            + evc_tbl_tm64[32][0] as i64 * src[32 * line + j] as i64;
        EEEEE[1] = evc_tbl_tm64[0][1] as i64 * src[0 * line + j] as i64
            + evc_tbl_tm64[32][1] as i64 * src[32 * line + j] as i64;

        for k in 0..2 {
            EEEE[k] = EEEEE[k] + EEEEO[k];
            EEEE[k + 2] = EEEEE[1 - k] - EEEEO[1 - k];
        }
        for k in 0..4 {
            EEE[k] = EEEE[k] + EEEO[k];
            EEE[k + 4] = EEEE[3 - k] - EEEO[3 - k];
        }
        for k in 0..8 {
            EE[k] = EEE[k] + EEO[k];
            EE[k + 8] = EEE[7 - k] - EEO[7 - k];
        }
        for k in 0..16 {
            E[k] = EE[k] + EO[k];
            E[k + 16] = EE[15 - k] - EO[15 - k];
        }

        for k in 0..32 {
            dst[j * 64 + k] = ITX_CLIP_32((E[k] + O[k] + add) >> shift as i64);
            dst[j * 64 + k + 32] = ITX_CLIP_32((E[31 - k] - O[31 - k] + add) >> shift as i64);
        }
    }
}

#[hawktracer(itx_pb2b1)]
fn itx_pb2b1(src: &[i32], dst: &mut [i16], shift: usize, line: usize) {
    let add = if shift == 0 {
        0
    } else {
        1 << (shift - 1) as i64
    };
    for j in 0..line {
        /* E and O */
        let E = src[0 * line + j] as i64 + src[1 * line + j] as i64;
        let O = src[0 * line + j] as i64 - src[1 * line + j] as i64;

        dst[j * 2 + 0] = ITX_CLIP((evc_tbl_tm2[0][0] as i64 * E + add) >> shift as i64);
        dst[j * 2 + 1] = ITX_CLIP((evc_tbl_tm2[1][0] as i64 * O + add) >> shift as i64);
    }
}

#[hawktracer(itx_pb4b1)]
fn itx_pb4b1(src: &[i32], dst: &mut [i16], shift: usize, line: usize) {
    let add = if shift == 0 {
        0
    } else {
        1 << (shift - 1) as i64
    };
    for j in 0..line {
        /* Utilizing symmetry properties to the maximum to minimize the number of multiplications */
        let O0 = evc_tbl_tm4[1][0] as i64 * src[1 * line + j] as i64
            + evc_tbl_tm4[3][0] as i64 * src[3 * line + j] as i64;
        let O1 = evc_tbl_tm4[1][1] as i64 * src[1 * line + j] as i64
            + evc_tbl_tm4[3][1] as i64 * src[3 * line + j] as i64;
        let E0 = evc_tbl_tm4[0][0] as i64 * src[0 * line + j] as i64
            + evc_tbl_tm4[2][0] as i64 * src[2 * line + j] as i64;
        let E1 = evc_tbl_tm4[0][1] as i64 * src[0 * line + j] as i64
            + evc_tbl_tm4[2][1] as i64 * src[2 * line + j] as i64;

        dst[j * 4 + 0] = ITX_CLIP((E0 + O0 + add) >> shift as i64);
        dst[j * 4 + 1] = ITX_CLIP((E1 + O1 + add) >> shift as i64);
        dst[j * 4 + 2] = ITX_CLIP((E1 - O1 + add) >> shift as i64);
        dst[j * 4 + 3] = ITX_CLIP((E0 - O0 + add) >> shift as i64);
    }
}

#[hawktracer(itx_pb8b1)]
fn itx_pb8b1(src: &[i32], dst: &mut [i16], shift: usize, line: usize) {
    let add = if shift == 0 {
        0
    } else {
        1 << (shift - 1) as i64
    };

    let mut E = [0i64; 4];
    let mut O = [0i64; 4];
    for j in 0..line {
        /* Utilizing symmetry properties to the maximum to minimize the number of multiplications */
        for k in 0..4 {
            O[k] = evc_tbl_tm8[1][k] as i64 * src[1 * line + j] as i64
                + evc_tbl_tm8[3][k] as i64 * src[3 * line + j] as i64
                + evc_tbl_tm8[5][k] as i64 * src[5 * line + j] as i64
                + evc_tbl_tm8[7][k] as i64 * src[7 * line + j] as i64;
        }

        let EO0 = evc_tbl_tm8[2][0] as i64 * src[2 * line + j] as i64
            + evc_tbl_tm8[6][0] as i64 * src[6 * line + j] as i64;
        let EO1 = evc_tbl_tm8[2][1] as i64 * src[2 * line + j] as i64
            + evc_tbl_tm8[6][1] as i64 * src[6 * line + j] as i64;
        let EE0 = evc_tbl_tm8[0][0] as i64 * src[0 * line + j] as i64
            + evc_tbl_tm8[4][0] as i64 * src[4 * line + j] as i64;
        let EE1 = evc_tbl_tm8[0][1] as i64 * src[0 * line + j] as i64
            + evc_tbl_tm8[4][1] as i64 * src[4 * line + j] as i64;

        /* Combining even and odd terms at each hierarchy levels to calculate the final spatial domain vector */
        E[0] = EE0 + EO0;
        E[3] = EE0 - EO0;
        E[1] = EE1 + EO1;
        E[2] = EE1 - EO1;

        for k in 0..4 {
            dst[j * 8 + k] = ITX_CLIP((E[k] + O[k] + add) >> shift as i64);
            dst[j * 8 + k + 4] = ITX_CLIP((E[3 - k] - O[3 - k] + add) >> shift as i64);
        }
    }
}

#[hawktracer(itx_pb16b1)]
fn itx_pb16b1(src: &[i32], dst: &mut [i16], shift: usize, line: usize) {
    let add = if shift == 0 {
        0
    } else {
        1 << (shift - 1) as i64
    };

    let mut E = [0i64; 8];
    let mut O = [0i64; 8];
    let mut EE = [0i64; 4];
    let mut EO = [0i64; 4];
    let mut EEE = [0i64; 2];
    let mut EEO = [0i64; 2];
    for j in 0..line {
        /* Utilizing symmetry properties to the maximum to minimize the number of multiplications */
        for k in 0..8 {
            O[k] = evc_tbl_tm16[1][k] as i64 * src[1 * line + j] as i64
                + evc_tbl_tm16[3][k] as i64 * src[3 * line + j] as i64
                + evc_tbl_tm16[5][k] as i64 * src[5 * line + j] as i64
                + evc_tbl_tm16[7][k] as i64 * src[7 * line + j] as i64
                + evc_tbl_tm16[9][k] as i64 * src[9 * line + j] as i64
                + evc_tbl_tm16[11][k] as i64 * src[11 * line + j] as i64
                + evc_tbl_tm16[13][k] as i64 * src[13 * line + j] as i64
                + evc_tbl_tm16[15][k] as i64 * src[15 * line + j] as i64;
        }

        for k in 0..4 {
            EO[k] = evc_tbl_tm16[2][k] as i64 * src[2 * line + j] as i64
                + evc_tbl_tm16[6][k] as i64 * src[6 * line + j] as i64
                + evc_tbl_tm16[10][k] as i64 * src[10 * line + j] as i64
                + evc_tbl_tm16[14][k] as i64 * src[14 * line + j] as i64;
        }

        EEO[0] = evc_tbl_tm16[4][0] as i64 * src[4 * line + j] as i64
            + evc_tbl_tm16[12][0] as i64 * src[12 * line + j] as i64;
        EEE[0] = evc_tbl_tm16[0][0] as i64 * src[0 * line + j] as i64
            + evc_tbl_tm16[8][0] as i64 * src[8 * line + j] as i64;
        EEO[1] = evc_tbl_tm16[4][1] as i64 * src[4 * line + j] as i64
            + evc_tbl_tm16[12][1] as i64 * src[12 * line + j] as i64;
        EEE[1] = evc_tbl_tm16[0][1] as i64 * src[0 * line + j] as i64
            + evc_tbl_tm16[8][1] as i64 * src[8 * line + j] as i64;

        /* Combining even and odd terms at each hierarchy levels to calculate the final spatial domain vector */
        for k in 0..2 {
            EE[k] = EEE[k] + EEO[k];
            EE[k + 2] = EEE[1 - k] - EEO[1 - k];
        }
        for k in 0..4 {
            E[k] = EE[k] + EO[k];
            E[k + 4] = EE[3 - k] - EO[3 - k];
        }

        for k in 0..8 {
            dst[j * 16 + k] = ITX_CLIP((E[k] + O[k] + add) >> shift as i64);
            dst[j * 16 + k + 8] = ITX_CLIP((E[7 - k] - O[7 - k] + add) >> shift as i64);
        }
    }
}

#[hawktracer(itx_pb32b1)]
fn itx_pb32b1(src: &[i32], dst: &mut [i16], shift: usize, line: usize) {
    let add = if shift == 0 {
        0
    } else {
        1 << (shift - 1) as i64
    };

    let mut E = [0i64; 16];
    let mut O = [0i64; 16];
    let mut EE = [0i64; 8];
    let mut EO = [0i64; 8];
    let mut EEE = [0i64; 4];
    let mut EEO = [0i64; 4];
    let mut EEEE = [0i64; 2];
    let mut EEEO = [0i64; 2];
    for j in 0..line {
        for k in 0..16 {
            O[k] = evc_tbl_tm32[1][k] as i64 * src[1 * line + j] as i64
                + evc_tbl_tm32[3][k] as i64 * src[3 * line + j] as i64
                + evc_tbl_tm32[5][k] as i64 * src[5 * line + j] as i64
                + evc_tbl_tm32[7][k] as i64 * src[7 * line + j] as i64
                + evc_tbl_tm32[9][k] as i64 * src[9 * line + j] as i64
                + evc_tbl_tm32[11][k] as i64 * src[11 * line + j] as i64
                + evc_tbl_tm32[13][k] as i64 * src[13 * line + j] as i64
                + evc_tbl_tm32[15][k] as i64 * src[15 * line + j] as i64
                + evc_tbl_tm32[17][k] as i64 * src[17 * line + j] as i64
                + evc_tbl_tm32[19][k] as i64 * src[19 * line + j] as i64
                + evc_tbl_tm32[21][k] as i64 * src[21 * line + j] as i64
                + evc_tbl_tm32[23][k] as i64 * src[23 * line + j] as i64
                + evc_tbl_tm32[25][k] as i64 * src[25 * line + j] as i64
                + evc_tbl_tm32[27][k] as i64 * src[27 * line + j] as i64
                + evc_tbl_tm32[29][k] as i64 * src[29 * line + j] as i64
                + evc_tbl_tm32[31][k] as i64 * src[31 * line + j] as i64;
        }

        for k in 0..8 {
            EO[k] = evc_tbl_tm32[2][k] as i64 * src[2 * line + j] as i64
                + evc_tbl_tm32[6][k] as i64 * src[6 * line + j] as i64
                + evc_tbl_tm32[10][k] as i64 * src[10 * line + j] as i64
                + evc_tbl_tm32[14][k] as i64 * src[14 * line + j] as i64
                + evc_tbl_tm32[18][k] as i64 * src[18 * line + j] as i64
                + evc_tbl_tm32[22][k] as i64 * src[22 * line + j] as i64
                + evc_tbl_tm32[26][k] as i64 * src[26 * line + j] as i64
                + evc_tbl_tm32[30][k] as i64 * src[30 * line + j] as i64;
        }

        for k in 0..4 {
            EEO[k] = evc_tbl_tm32[4][k] as i64 * src[4 * line + j] as i64
                + evc_tbl_tm32[12][k] as i64 * src[12 * line + j] as i64
                + evc_tbl_tm32[20][k] as i64 * src[20 * line + j] as i64
                + evc_tbl_tm32[28][k] as i64 * src[28 * line + j] as i64;
        }

        EEEO[0] = evc_tbl_tm32[8][0] as i64 * src[8 * line + j] as i64
            + evc_tbl_tm32[24][0] as i64 * src[24 * line + j] as i64;
        EEEO[1] = evc_tbl_tm32[8][1] as i64 * src[8 * line + j] as i64
            + evc_tbl_tm32[24][1] as i64 * src[24 * line + j] as i64;
        EEEE[0] = evc_tbl_tm32[0][0] as i64 * src[0 * line + j] as i64
            + evc_tbl_tm32[16][0] as i64 * src[16 * line + j] as i64;
        EEEE[1] = evc_tbl_tm32[0][1] as i64 * src[0 * line + j] as i64
            + evc_tbl_tm32[16][1] as i64 * src[16 * line + j] as i64;

        EEE[0] = EEEE[0] + EEEO[0];
        EEE[3] = EEEE[0] - EEEO[0];
        EEE[1] = EEEE[1] + EEEO[1];
        EEE[2] = EEEE[1] - EEEO[1];
        for k in 0..4 {
            EE[k] = EEE[k] + EEO[k];
            EE[k + 4] = EEE[3 - k] - EEO[3 - k];
        }
        for k in 0..8 {
            E[k] = EE[k] + EO[k];
            E[k + 8] = EE[7 - k] - EO[7 - k];
        }

        for k in 0..16 {
            dst[j * 32 + k] = ITX_CLIP((E[k] + O[k] + add) >> shift as i64);
            dst[j * 32 + k + 16] = ITX_CLIP((E[15 - k] - O[15 - k] + add) >> shift as i64);
        }
    }
}

#[hawktracer(itx_pb64b1)]
fn itx_pb64b1(src: &[i32], dst: &mut [i16], shift: usize, line: usize) {
    let add = if shift == 0 {
        0
    } else {
        1 << (shift - 1) as i64
    };
    let mut E = [0i64; 32];
    let mut O = [0i64; 32];
    let mut EE = [0i64; 16];
    let mut EO = [0i64; 16];
    let mut EEE = [0i64; 8];
    let mut EEO = [0i64; 8];
    let mut EEEE = [0i64; 4];
    let mut EEEO = [0i64; 4];
    let mut EEEEE = [0i64; 2];
    let mut EEEEO = [0i64; 2];
    for j in 0..line {
        for k in 0..32 {
            O[k] = evc_tbl_tm64[1][k] as i64 * src[1 * line + j] as i64
                + evc_tbl_tm64[3][k] as i64 * src[3 * line + j] as i64
                + evc_tbl_tm64[5][k] as i64 * src[5 * line + j] as i64
                + evc_tbl_tm64[7][k] as i64 * src[7 * line + j] as i64
                + evc_tbl_tm64[9][k] as i64 * src[9 * line + j] as i64
                + evc_tbl_tm64[11][k] as i64 * src[11 * line + j] as i64
                + evc_tbl_tm64[13][k] as i64 * src[13 * line + j] as i64
                + evc_tbl_tm64[15][k] as i64 * src[15 * line + j] as i64
                + evc_tbl_tm64[17][k] as i64 * src[17 * line + j] as i64
                + evc_tbl_tm64[19][k] as i64 * src[19 * line + j] as i64
                + evc_tbl_tm64[21][k] as i64 * src[21 * line + j] as i64
                + evc_tbl_tm64[23][k] as i64 * src[23 * line + j] as i64
                + evc_tbl_tm64[25][k] as i64 * src[25 * line + j] as i64
                + evc_tbl_tm64[27][k] as i64 * src[27 * line + j] as i64
                + evc_tbl_tm64[29][k] as i64 * src[29 * line + j] as i64
                + evc_tbl_tm64[31][k] as i64 * src[31 * line + j] as i64
                + evc_tbl_tm64[33][k] as i64 * src[33 * line + j] as i64
                + evc_tbl_tm64[35][k] as i64 * src[35 * line + j] as i64
                + evc_tbl_tm64[37][k] as i64 * src[37 * line + j] as i64
                + evc_tbl_tm64[39][k] as i64 * src[39 * line + j] as i64
                + evc_tbl_tm64[41][k] as i64 * src[41 * line + j] as i64
                + evc_tbl_tm64[43][k] as i64 * src[43 * line + j] as i64
                + evc_tbl_tm64[45][k] as i64 * src[45 * line + j] as i64
                + evc_tbl_tm64[47][k] as i64 * src[47 * line + j] as i64
                + evc_tbl_tm64[49][k] as i64 * src[49 * line + j] as i64
                + evc_tbl_tm64[51][k] as i64 * src[51 * line + j] as i64
                + evc_tbl_tm64[53][k] as i64 * src[53 * line + j] as i64
                + evc_tbl_tm64[55][k] as i64 * src[55 * line + j] as i64
                + evc_tbl_tm64[57][k] as i64 * src[57 * line + j] as i64
                + evc_tbl_tm64[59][k] as i64 * src[59 * line + j] as i64
                + evc_tbl_tm64[61][k] as i64 * src[61 * line + j] as i64
                + evc_tbl_tm64[63][k] as i64 * src[63 * line + j] as i64;
        }

        for k in 0..16 {
            EO[k] = evc_tbl_tm64[2][k] as i64 * src[2 * line + j] as i64
                + evc_tbl_tm64[6][k] as i64 * src[6 * line + j] as i64
                + evc_tbl_tm64[10][k] as i64 * src[10 * line + j] as i64
                + evc_tbl_tm64[14][k] as i64 * src[14 * line + j] as i64
                + evc_tbl_tm64[18][k] as i64 * src[18 * line + j] as i64
                + evc_tbl_tm64[22][k] as i64 * src[22 * line + j] as i64
                + evc_tbl_tm64[26][k] as i64 * src[26 * line + j] as i64
                + evc_tbl_tm64[30][k] as i64 * src[30 * line + j] as i64
                + evc_tbl_tm64[34][k] as i64 * src[34 * line + j] as i64
                + evc_tbl_tm64[38][k] as i64 * src[38 * line + j] as i64
                + evc_tbl_tm64[42][k] as i64 * src[42 * line + j] as i64
                + evc_tbl_tm64[46][k] as i64 * src[46 * line + j] as i64
                + evc_tbl_tm64[50][k] as i64 * src[50 * line + j] as i64
                + evc_tbl_tm64[54][k] as i64 * src[54 * line + j] as i64
                + evc_tbl_tm64[58][k] as i64 * src[58 * line + j] as i64
                + evc_tbl_tm64[62][k] as i64 * src[62 * line + j] as i64;
        }

        for k in 0..8 {
            EEO[k] = evc_tbl_tm64[4][k] as i64 * src[4 * line + j] as i64
                + evc_tbl_tm64[12][k] as i64 * src[12 * line + j] as i64
                + evc_tbl_tm64[20][k] as i64 * src[20 * line + j] as i64
                + evc_tbl_tm64[28][k] as i64 * src[28 * line + j] as i64
                + evc_tbl_tm64[36][k] as i64 * src[36 * line + j] as i64
                + evc_tbl_tm64[44][k] as i64 * src[44 * line + j] as i64
                + evc_tbl_tm64[52][k] as i64 * src[52 * line + j] as i64
                + evc_tbl_tm64[60][k] as i64 * src[60 * line + j] as i64;
        }

        for k in 0..4 {
            EEEO[k] = evc_tbl_tm64[8][k] as i64 * src[8 * line + j] as i64
                + evc_tbl_tm64[24][k] as i64 * src[24 * line + j] as i64
                + evc_tbl_tm64[40][k] as i64 * src[40 * line + j] as i64
                + evc_tbl_tm64[56][k] as i64 * src[56 * line + j] as i64;
        }
        EEEEO[0] = evc_tbl_tm64[16][0] as i64 * src[16 * line + j] as i64
            + evc_tbl_tm64[48][0] as i64 * src[48 * line + j] as i64;
        EEEEO[1] = evc_tbl_tm64[16][1] as i64 * src[16 * line + j] as i64
            + evc_tbl_tm64[48][1] as i64 * src[48 * line + j] as i64;
        EEEEE[0] = evc_tbl_tm64[0][0] as i64 * src[0 * line + j] as i64
            + evc_tbl_tm64[32][0] as i64 * src[32 * line + j] as i64;
        EEEEE[1] = evc_tbl_tm64[0][1] as i64 * src[0 * line + j] as i64
            + evc_tbl_tm64[32][1] as i64 * src[32 * line + j] as i64;

        for k in 0..2 {
            EEEE[k] = EEEEE[k] + EEEEO[k];
            EEEE[k + 2] = EEEEE[1 - k] - EEEEO[1 - k];
        }
        for k in 0..4 {
            EEE[k] = EEEE[k] + EEEO[k];
            EEE[k + 4] = EEEE[3 - k] - EEEO[3 - k];
        }
        for k in 0..8 {
            EE[k] = EEE[k] + EEO[k];
            EE[k + 8] = EEE[7 - k] - EEO[7 - k];
        }
        for k in 0..16 {
            E[k] = EE[k] + EO[k];
            E[k + 16] = EE[15 - k] - EO[15 - k];
        }

        for k in 0..32 {
            dst[j * 64 + k] = ITX_CLIP((E[k] + O[k] + add) >> shift as i64);
            dst[j * 64 + k + 32] = ITX_CLIP((E[31 - k] - O[31 - k] + add) >> shift as i64);
        }
    }
}

type EVC_ITXB0 = fn(src: &[i16], dst: &mut [i32], shift: usize, line: usize);
type EVC_ITXB1 = fn(src: &[i32], dst: &mut [i16], shift: usize, line: usize);

static tbl_itxb0: [EVC_ITXB0; MAX_TR_LOG2] = [
    itx_pb2b0, itx_pb4b0, itx_pb8b0, itx_pb16b0, itx_pb32b0, itx_pb64b0,
];
static tbl_itxb1: [EVC_ITXB1; MAX_TR_LOG2] = [
    itx_pb2b1, itx_pb4b1, itx_pb8b1, itx_pb16b1, itx_pb32b1, itx_pb64b1,
];

#[hawktracer(evc_itrans)]
fn evc_itrans(coef: &mut [i16], log2_cuw: usize, log2_cuh: usize) {
    let mut tb = [0i32; MAX_TR_DIM]; /* temp buffer */
    tbl_itxb0[log2_cuh - 1](coef, &mut tb, 0, 1 << log2_cuw);
    tbl_itxb1[log2_cuw - 1](&tb, coef, (ITX_SHIFT1 + ITX_SHIFT2), 1 << log2_cuh);

    //alternative way
    //tbl_itxb[log2_cuw - 1](coef, 0, (ITX_SHIFT1 + ITX_SHIFT2), 1 << log2_cuw);
}

#[hawktracer(evc_itdq)]
fn evc_itdq(coef: &mut [i16], log2_w: usize, log2_h: usize, scale: i32) {
    let log2_size = (log2_w + log2_h) >> 1;
    let ns_shift = if (log2_w + log2_h) & 1 != 0 { 8 } else { 0 };

    let tr_shift: i8 = MAX_TX_DYNAMIC_RANGE as i8 - BIT_DEPTH as i8 - log2_size as i8;
    let shift: u8 = (QUANT_IQUANT_SHIFT as i8 - QUANT_SHIFT as i8 - tr_shift + ns_shift) as u8;
    let offset: i32 = if shift == 0 {
        0
    } else {
        1 << (shift as i32 - 1)
    };

    evc_dquant(coef, log2_w, log2_h, scale, offset, shift);
    evc_itrans(coef, log2_w, log2_h);
}
