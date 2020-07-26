use super::*;
use crate::api::*;
use crate::def::*;
use crate::plane::*;

#[derive(Default)]
pub(crate) struct EvceCUData {
    split_mode: LcuSplitMode,
    qp_y: Vec<u8>,
    qp_u: Vec<u8>,
    qp_v: Vec<u8>,
    pred_mode: Vec<PredMode>,
    pred_mode_chroma: Vec<PredMode>,
    ipm: Vec<Vec<IntraPredDir>>,
    skip_flag: Vec<bool>,
    refi: Vec<Vec<i8>>,
    mvp_idx: Vec<Vec<u8>>,
    mv: Vec<Vec<Vec<i16>>>,  //[MAX_CU_CNT_IN_LCU][REFP_NUM][MV_D];
    mvd: Vec<Vec<Vec<i16>>>, //[MAX_CU_CNT_IN_LCU][REFP_NUM][MV_D];
    nnz: Vec<Vec<bool>>,     //[N_C];
    map_scu: Vec<MCU>,
    map_cu_mode: Vec<MCU>,
    depth: Vec<i8>,
    coef: Vec<Vec<i16>>, //[N_C];
    reco: Vec<Vec<pel>>, //[N_C];
                         //#if TRACE_ENC_CU_DATA
                         //  u64  trace_idx[MAX_CU_CNT_IN_LCU];
                         //#endif
}

impl EvceCUData {
    pub(crate) fn new(log2_cuw: u8, log2_cuh: u8) -> Self {
        let cuw_scu = 1 << log2_cuw as usize;
        let cuh_scu = 1 << log2_cuh as usize;

        let cu_cnt = cuw_scu * cuh_scu;
        let pixel_cnt = cu_cnt << 4;

        let mut coef = Vec::with_capacity(N_C);
        let mut reco = Vec::with_capacity(N_C);
        for i in 0..N_C {
            let chroma = if i > 0 { 1 } else { 0 };
            coef.push(vec![0; pixel_cnt >> (chroma * 2)]);
            reco.push(vec![0; pixel_cnt >> (chroma * 2)]);
        }

        EvceCUData {
            split_mode: LcuSplitMode::default(),
            qp_y: vec![0; cu_cnt],
            qp_u: vec![0; cu_cnt],
            qp_v: vec![0; cu_cnt],
            pred_mode: vec![PredMode::MODE_INTRA; cu_cnt],
            pred_mode_chroma: vec![PredMode::MODE_INTRA; cu_cnt],
            ipm: vec![vec![IntraPredDir::IPD_DC_B; cu_cnt]; 2],
            skip_flag: vec![false; cu_cnt],
            refi: vec![vec![0; REFP_NUM]; cu_cnt],
            mvp_idx: vec![vec![0; REFP_NUM]; cu_cnt],
            mv: vec![vec![vec![0; MV_D]; REFP_NUM]; cu_cnt],
            mvd: vec![vec![vec![0; MV_D]; REFP_NUM]; cu_cnt],
            nnz: vec![vec![false; cu_cnt]; N_C],
            map_scu: vec![MCU::default(); cu_cnt],
            map_cu_mode: vec![MCU::default(); cu_cnt],
            depth: vec![0; cu_cnt],
            coef,
            reco,
        }
    }
    pub(crate) fn init(&mut self, log2_cuw: u8, log2_cuh: u8, qp_y: u8, qp_u: u8, qp_v: u8) {
        let cuw_scu = 1 << log2_cuw as usize;
        let cuh_scu = 1 << log2_cuh as usize;

        let cu_cnt = cuw_scu * cuh_scu;
        let pixel_cnt = cu_cnt << 4;

        for i in 0..NUM_CU_DEPTH {
            for j in 0..BlockShape::NUM_BLOCK_SHAPE as usize {
                for v in &mut self.split_mode.data[i][j] {
                    *v = SplitMode::NO_SPLIT;
                }
            }
        }

        for i in 0..cu_cnt {
            self.qp_y[i] = 0;
            self.qp_u[i] = 0;
            self.qp_v[i] = 0;
            self.ipm[0][i] = IntraPredDir::IPD_DC_B;
            self.ipm[1][i] = IntraPredDir::IPD_DC_B;
        }
    }

    pub(crate) fn copy(
        &mut self,
        src: &EvceCUData,
        x: u16,
        y: u16,
        log2_cuw: u8,
        log2_cuh: u8,
        log2_cus: u8,
        cud: u16,
        tree_cons: &TREE_CONS,
    ) {
        let cx = x as usize >> MIN_CU_LOG2; //x = position in LCU, cx = 4x4 CU horizontal index
        let cy = y as usize >> MIN_CU_LOG2; //y = position in LCU, cy = 4x4 CU vertical index

        let cuw = (1 << log2_cuw) as usize; //current CU width
        let cuh = (1 << log2_cuh) as usize; //current CU height
        let cus = (1 << log2_cus) as usize; //current CU buffer stride (= current CU width)
        let cuw_scu = (1 << log2_cuw) as usize - MIN_CU_LOG2; //4x4 CU number in width
        let cuh_scu = (1 << log2_cuh) as usize - MIN_CU_LOG2; //4x4 CU number in height
        let cus_scu = (1 << log2_cus) as usize - MIN_CU_LOG2; //4x4 CU number in stride

        // only copy src's first row of 4x4 CUs to dis's all 4x4 CUs
        if evc_check_luma(tree_cons) {
            let size = cuw_scu;
            for j in 0..cuh_scu {
                let idx_dst = (cy + j) * cus_scu + cx;
                let idx_src = j * cuw_scu;

                for k in cud as usize..NUM_CU_DEPTH {
                    for i in 0..BlockShape::NUM_BLOCK_SHAPE as usize {
                        self.split_mode.data[k][i][idx_dst..idx_dst + size]
                            .copy_from_slice(&src.split_mode.data[k][i][idx_src..idx_src + size]);
                    }
                }

                self.qp_y[idx_dst..idx_dst + size]
                    .copy_from_slice(&src.qp_y[idx_src..idx_src + size]);
                self.pred_mode[idx_dst..idx_dst + size]
                    .copy_from_slice(&src.pred_mode[idx_src..idx_src + size]);
                self.ipm[0][idx_dst..idx_dst + size]
                    .copy_from_slice(&src.ipm[0][idx_src..idx_src + size]);
                self.skip_flag[idx_dst..idx_dst + size]
                    .copy_from_slice(&src.skip_flag[idx_src..idx_src + size]);
                self.depth[idx_dst..idx_dst + size]
                    .copy_from_slice(&src.depth[idx_src..idx_src + size]);
                self.map_scu[idx_dst..idx_dst + size]
                    .copy_from_slice(&src.map_scu[idx_src..idx_src + size]);
                self.map_cu_mode[idx_dst..idx_dst + size]
                    .copy_from_slice(&src.map_cu_mode[idx_src..idx_src + size]);
                self.refi[idx_dst..idx_dst + size]
                    .clone_from_slice(&src.refi[idx_src..idx_src + size]);
                self.mvp_idx[idx_dst..idx_dst + size]
                    .clone_from_slice(&src.mvp_idx[idx_src..idx_src + size]);
                self.mv[idx_dst..idx_dst + size].clone_from_slice(&src.mv[idx_src..idx_src + size]);
                self.mvd[idx_dst..idx_dst + size]
                    .clone_from_slice(&src.mvd[idx_src..idx_src + size]);
                self.nnz[Y_C][idx_dst..idx_dst + size]
                    .copy_from_slice(&src.nnz[Y_C][idx_src..idx_src + size]);

                //#if TRACE_ENC_CU_DATA
                //        size = cuw_scu * sizeof(dst->trace_idx[0]);
                //        evc_mcpy(dst->trace_idx + idx_dst, src->trace_idx + idx_src, size);
                //#endif
            }

            let size = cuw;
            for j in 0..cuh {
                let idx_dst = (y as usize + j) * cus + x as usize;
                let idx_src = j * cuw;

                self.coef[Y_C][idx_dst..idx_dst + size]
                    .copy_from_slice(&src.coef[Y_C][idx_src..idx_src + size]);
                self.reco[Y_C][idx_dst..idx_dst + size]
                    .copy_from_slice(&src.reco[Y_C][idx_src..idx_src + size]);
            }
        }

        if evc_check_chroma(tree_cons) {
            let size = cuw >> 1;
            for j in 0..cuh >> 1 {
                let idx_dst = ((y >> 1) as usize + j) * (cus >> 1) + (x >> 1) as usize;
                let idx_src = j * (cuw >> 1);

                self.coef[U_C][idx_dst..idx_dst + size]
                    .copy_from_slice(&src.coef[U_C][idx_src..idx_src + size]);
                self.reco[U_C][idx_dst..idx_dst + size]
                    .copy_from_slice(&src.reco[U_C][idx_src..idx_src + size]);

                self.coef[V_C][idx_dst..idx_dst + size]
                    .copy_from_slice(&src.coef[V_C][idx_src..idx_src + size]);
                self.reco[V_C][idx_dst..idx_dst + size]
                    .copy_from_slice(&src.reco[V_C][idx_src..idx_src + size]);
            }

            let size = cuw_scu;
            for j in 0..cuh_scu {
                let idx_dst = (cy + j) * cus_scu + cx;
                let idx_src = j * cuw_scu;

                self.qp_u[idx_dst..idx_dst + size]
                    .copy_from_slice(&src.qp_u[idx_src..idx_src + size]);
                self.qp_v[idx_dst..idx_dst + size]
                    .copy_from_slice(&src.qp_v[idx_src..idx_src + size]);

                self.ipm[1][idx_dst..idx_dst + size]
                    .copy_from_slice(&src.ipm[1][idx_src..idx_src + size]);
                self.pred_mode_chroma[idx_dst..idx_dst + size]
                    .copy_from_slice(&src.pred_mode_chroma[idx_src..idx_src + size]);

                self.nnz[U_C][idx_dst..idx_dst + size]
                    .copy_from_slice(&src.nnz[U_C][idx_src..idx_src + size]);
                self.nnz[V_C][idx_dst..idx_dst + size]
                    .copy_from_slice(&src.nnz[V_C][idx_src..idx_src + size]);
            }
        }
    }

    fn mode_cpy_rec_to_ref(
        &mut self,
        mut x: usize,
        mut y: usize,
        mut w: usize,
        mut h: usize,
        planes: &mut [Plane<pel>; N_C],
        tree_cons: &TREE_CONS,
    ) {
        if evc_check_luma(tree_cons) {
            /* luma */
            let dst = &mut planes[Y_C].as_region_mut();
            let src = &self.reco[Y_C];

            for j in 0..h {
                for i in 0..w {
                    dst[y + j][x + i] = src[j * w + i];
                }
            }
        }

        if evc_check_chroma(tree_cons) {
            /* chroma */
            x >>= 1;
            y >>= 1;
            w >>= 1;
            h >>= 1;

            {
                let dst = &mut planes[U_C].as_region_mut();
                let src = &self.reco[U_C];

                for j in 0..h {
                    for i in 0..w {
                        dst[y + j][x + i] = src[j * w + i];
                    }
                }
            }

            {
                let dst = &mut planes[V_C].as_region_mut();
                let src = &self.reco[V_C];

                for j in 0..h {
                    for i in 0..w {
                        dst[y + j][x + i] = src[j * w + i];
                    }
                }
            }
        }
    }
}

/*****************************************************************************
 * mode decision structure
 *****************************************************************************/
#[derive(Default)]
pub(crate) struct EvceMode {
    //void *pdata[4];
    //int  *ndata[4];
    //pel  *rec[N_C];
    //int   s_rec[N_C];

    /* CU count in a CU row in a LCU (== log2_max_cuwh - MIN_CU_LOG2) */
    log2_culine: u8,
    /* reference indices */
    refi: [i8; REFP_NUM],
    /* MVP indices */
    mvp_idx: [u8; REFP_NUM],
    /* MVR indices */
    //u8    mvr_idx;
    bi_idx: u8,
    /* mv difference */
    mvd: [[i16; MV_D]; REFP_NUM],

    /* mv */
    mv: [[i16; MV_D]; REFP_NUM],

    //pel  *pred_y_best;
    cu_mode: MCU,
}

impl EvceMode {
    fn get_cu_pred_data(
        &mut self,
        src: &EvceCUData,
        x: u16,
        y: u16,
        log2_cuw: u8,
        log2_cuh: u8,
        log2_cus: u8,
        cud: u16,
    ) {
        let cx = x as usize >> MIN_CU_LOG2; //x = position in LCU, cx = 4x4 CU horizontal index
        let cy = y as usize >> MIN_CU_LOG2; //y = position in LCU, cy = 4x4 CU vertical index

        let cuw = (1 << log2_cuw) as usize; //current CU width
        let cuh = (1 << log2_cuh) as usize; //current CU height
        let cus = (1 << log2_cus) as usize; //current CU buffer stride (= current CU width)
        let cuw_scu = (1 << log2_cuw) as usize - MIN_CU_LOG2; //4x4 CU number in width
        let cuh_scu = (1 << log2_cuh) as usize - MIN_CU_LOG2; //4x4 CU number in height
        let cus_scu = (1 << log2_cus) as usize - MIN_CU_LOG2; //4x4 CU number in stride

        // only copy src's first row of 4x4 CUs to dis's all 4x4 CUs
        let idx_src = cy * cus_scu + cx;

        self.cu_mode = (src.pred_mode[idx_src] as u32).into();
        self.mv[REFP_0][MV_X] = src.mv[idx_src][REFP_0][MV_X];
        self.mv[REFP_0][MV_Y] = src.mv[idx_src][REFP_0][MV_Y];
        self.mv[REFP_1][MV_X] = src.mv[idx_src][REFP_1][MV_X];
        self.mv[REFP_1][MV_Y] = src.mv[idx_src][REFP_1][MV_Y];

        self.refi[REFP_0] = src.refi[idx_src][REFP_0];
        self.refi[REFP_1] = src.refi[idx_src][REFP_1];

        /*#if TRACE_ENC_CU_DATA
            mi->trace_cu_idx = src->trace_idx[idx_src];
        #endif
        #if TRACE_ENC_CU_DATA_CHECK
            evc_assert(mi->trace_cu_idx != 0);
        #endif*/
    }
}

impl EvceCtx {
    pub(crate) fn mode_init_frame(&mut self) {
        let mi = &mut self.mode;
        /* set default values to mode information */
        mi.log2_culine = self.log2_max_cuwh - MIN_CU_LOG2 as u8;

        self.pintra_init_frame();
        self.pinter_init_frame();
    }

    pub(crate) fn mode_analyze_frame(&mut self) {
        self.pintra_analyze_frame();
        self.pinter_analyze_frame();
    }

    pub(crate) fn mode_init_lcu(&mut self) {
        self.pintra_init_lcu();
        self.pinter_init_lcu();
    }

    pub(crate) fn mode_analyze_lcu(&mut self) {
        let mut split_mode_child = [false, false, false, false]; //&mut self.core.split_mode_child;
        let mut parent_split_allow = [false, false, false, false, false, true]; //&mut self.core.parent_split_allow;

        let mi = &mut self.mode;

        /* initialize cu data */
        self.core.cu_data_best[self.log2_max_cuwh as usize - 2][self.log2_max_cuwh as usize - 2]
            .init(
                self.log2_max_cuwh,
                self.log2_max_cuwh,
                self.qp,
                self.qp,
                self.qp,
            );
        self.core.cu_data_temp[self.log2_max_cuwh as usize - 2][self.log2_max_cuwh as usize - 2]
            .init(
                self.log2_max_cuwh,
                self.log2_max_cuwh,
                self.qp,
                self.qp,
                self.qp,
            );

        for i in 0..REFP_NUM {
            mi.mvp_idx[i] = 0;
        }
        for i in 0..REFP_NUM {
            for j in 0..MV_D {
                mi.mvd[i][j] = 0;
            }
        }

        /* decide mode */
        self.mode_coding_tree(
            self.core.x_pel,
            self.core.y_pel,
            0,
            self.log2_max_cuwh,
            self.log2_max_cuwh,
            0,
            true,
            SplitMode::NO_SPLIT,
            &mut split_mode_child,
            0,
            &mut parent_split_allow,
            0,
            self.qp,
            evc_get_default_tree_cons(),
        );

        /*#if TRACE_ENC_CU_DATA_CHECK
                let h = 1 << (self.log2_max_cuwh - MIN_CU_LOG2);
                    let w = 1 << (self.log2_max_cuwh - MIN_CU_LOG2);
                for j in 0..h {
                    let y_pos = self.core.y_pel + (j << MIN_CU_LOG2);
                    for i in 0..w {
                        let x_pos = self.core.x_pel + (i << MIN_CU_LOG2);
                        if x_pos < self.w && y_pos < self.h {
                            assert!(self.core.cu_data_best
                            [self.log2_max_cuwh as usize - 2][self.log2_max_cuwh as usize- 2].trace_idx[i + h * j] != 0);
                        }
                    }
                }
        #endif*/
    }

    fn mode_coding_tree(
        &mut self,
        x0: u16,
        y0: u16,
        cup: u16,
        log2_cuw: u8,
        log2_cuh: u8,
        cud: u16,
        next_split: bool,
        parent_split: SplitMode,
        same_layer_split: &mut [bool],
        node_idx: usize,
        parent_split_allow: &mut [bool],
        qt_depth: u8,
        qp: u8,
        tree_cons: TREE_CONS,
    ) {
        // x0 = CU's left up corner horizontal index in entrie frame
        // y0 = CU's left up corner vertical index in entire frame
        // cuw = CU width, log2_cuw = CU width in log2
        // cuh = CU height, log2_chu = CU height in log2
        // self.w = frame width, self.h = frame height
        let cuw = 1 << log2_cuw;
        let cuh = 1 << log2_cuh;
        let mut best_split_mode = SplitMode::NO_SPLIT;
        let mut bit_cnt = 0;
        let mut cost_best = MAX_COST;
        let mut cost_temp = MAX_COST;
        let mut s_temp_depth = EvceSbac::default();
        let boundary = !(x0 + cuw <= self.w && y0 + cuh <= self.h);
        let mut split_allow = vec![false; MAX_SPLIT_NUM];
        let avail_lr = evc_check_nev_avail(
            PEL2SCU(x0 as usize) as u16,
            PEL2SCU(y0 as usize) as u16,
            cuw,
            self.w_scu,
            &self.map_scu,
        );
        let mut split_mode = SplitMode::NO_SPLIT;
        let mut do_split = false;
        let mut do_curr = false;
        let best_split_cost = MAX_COST;
        let best_curr_cost = MAX_COST;
        let split_mode_child = [
            SplitMode::NO_SPLIT,
            SplitMode::NO_SPLIT,
            SplitMode::NO_SPLIT,
            SplitMode::NO_SPLIT,
        ];
        let mut curr_split_allow = vec![false; MAX_SPLIT_NUM];
        let remaining_split = 0;
        let num_split_tried = 0;
        let mut num_split_to_try = 0;
        let mut nev_max_depth = 0;
        let eval_parent_node_first = 1;
        let mut nbr_map_skip_flag = false;
        let cud_min = cud;
        let cud_max = cud;
        let cud_avg = cud;
        let mut dqp_temp_depth = EvceDQP::default();
        let mut best_dqp = qp;
        let mut min_qp = 0;
        let mut max_qp = 0;
        let mut cost_temp_dqp = 0.0f64;
        let mut cost_best_dqp = MAX_COST;
        let mut dqp_coded = 0;
        let mut cu_mode_dqp = MCU::default();
        let mut dist_cu_best_dqp = 0;

        self.core.tree_cons = tree_cons;
        self.core.avail_lr = avail_lr;

        self.core.s_curr_before_split[log2_cuw as usize - 2][log2_cuh as usize - 2] =
            self.core.s_curr_best[log2_cuw as usize - 2][log2_cuh as usize - 2];

        //decide allowed split modes for the current node
        //based on CU size located at boundary
        if cuw > self.min_cuwh || cuh > self.min_cuwh {
            /***************************** Step 1: decide normatively allowed split modes ********************************/
            let boundary_b = boundary && (y0 + cuh > self.h) && !(x0 + cuw > self.w);
            let boundary_r = boundary && (x0 + cuw > self.w) && !(y0 + cuh > self.h);
            evc_check_split_mode(&mut split_allow);
            //save normatively allowed split modes, as it will be used in in child nodes for entropy coding of split mode
            curr_split_allow.copy_from_slice(&split_allow);
            for i in 1..MAX_SPLIT_NUM {
                num_split_to_try += if split_allow[i] { 1 } else { 0 };
            }

            /***************************** Step 2: reduce split modes by fast algorithm ********************************/
            do_split = true;
            do_curr = true;
            if !boundary {
                assert!(evc_check_luma(&self.core.tree_cons));
                nev_max_depth = self.check_nev_block(
                    x0,
                    y0,
                    log2_cuw,
                    log2_cuh,
                    &mut do_curr,
                    &mut do_split,
                    cud,
                    &mut nbr_map_skip_flag,
                );
                do_split = true;
                do_curr = true;
            }

            self.check_run_split(
                log2_cuw,
                log2_cuh,
                cup,
                next_split,
                do_curr,
                do_split,
                &mut split_allow,
                boundary,
                &tree_cons,
            );
        } else {
            split_allow[0] = true;
            for i in 1..MAX_SPLIT_NUM {
                split_allow[i] = false;
            }
        }

        if !boundary {
            cost_temp = 0.0;

            self.core.cu_data_temp[log2_cuw as usize - 2][log2_cuh as usize - 2]
                .init(log2_cuw, log2_cuh, self.qp, self.qp, self.qp);

            self.sh.qp_prev_mode =
                self.core.dqp_data[log2_cuw as usize - 2][log2_cuh as usize - 2].prev_QP as u8;
            best_dqp = self.sh.qp_prev_mode;

            split_mode = SplitMode::NO_SPLIT;
            if split_allow[split_mode as usize] {
                if (cuw > self.min_cuwh || cuh > self.min_cuwh)
                    && evc_check_luma(&self.core.tree_cons)
                {
                    /* consider CU split mode */
                    self.core.s_temp_run =
                        self.core.s_curr_best[log2_cuw as usize - 2][log2_cuh as usize - 2];
                    self.core.s_temp_run.bit_reset();
                    evc_set_split_mode(
                        &mut self.core.cu_data_temp[log2_cuw as usize - 2][log2_cuh as usize - 2]
                            .split_mode,
                        SplitMode::NO_SPLIT,
                        cud,
                        0,
                        cuw,
                        cuh,
                        cuw,
                    );
                    let split_mode_buf = if self.core.s_temp_run.is_bitcount {
                        &self.core.cu_data_temp[CONV_LOG2(cuw as usize) as usize - 2]
                            [CONV_LOG2(cuh as usize) as usize - 2]
                            .split_mode
                    } else {
                        &self.map_cu_data[self.core.lcu_num as usize].split_mode
                    };
                    evce_eco_split_mode(
                        &mut self.core.bs_temp,
                        &mut self.core.s_temp_run,
                        &mut self.core.c_temp_run,
                        cud,
                        0,
                        cuw,
                        cuh,
                        cuw,
                        split_mode_buf,
                    );

                    bit_cnt = self.core.s_temp_run.get_bit_number();
                    cost_temp += self.lambda[0] * bit_cnt as f64;
                    self.core.s_curr_best[log2_cuw as usize - 2][log2_cuh as usize - 2] =
                        self.core.s_temp_run;
                }

                self.core.cup = cup as u32;
                let mut is_dqp_set = false;
                self.get_min_max_qp(
                    &mut min_qp,
                    &mut max_qp,
                    &mut is_dqp_set,
                    split_mode,
                    cuw,
                    cuh,
                    qp,
                    x0,
                    y0,
                );
                for dqp in min_qp..=max_qp {
                    self.core.qp = GET_QP(qp as i8, dqp as i8 - qp as i8) as u8;
                    self.core.dqp_curr_best[log2_cuw as usize - 2][log2_cuh as usize - 2].curr_QP =
                        self.core.qp as i8;
                    if self.core.cu_qp_delta_code_mode != 2 || is_dqp_set {
                        self.core.dqp_curr_best[log2_cuw as usize - 2][log2_cuh as usize - 2]
                            .cu_qp_delta_code = 1 + if is_dqp_set { 1 } else { 0 };
                        self.core.dqp_curr_best[log2_cuw as usize - 2][log2_cuh as usize - 2]
                            .cu_qp_delta_is_coded = false;
                    }
                    cost_temp_dqp = cost_temp;
                    self.core.cu_data_temp[log2_cuw as usize - 2][log2_cuh as usize - 2]
                        .init(log2_cuw, log2_cuh, self.qp, self.qp, self.qp);

                    self.clear_map_scu(x0, y0, cuw, cuh);
                    /*cost_temp_dqp += mode_coding_unit(ctx, core, x0, y0, log2_cuw, log2_cuh, cud, mi);
                     */
                    if cost_best > cost_temp_dqp {
                        cu_mode_dqp = self.core.cu_mode;
                        dist_cu_best_dqp = self.core.dist_cu_best;
                        /* backup the current best data */
                        self.core.cu_data_best[log2_cuw as usize - 2][log2_cuh as usize - 2].copy(
                            &self.core.cu_data_temp[log2_cuw as usize - 2][log2_cuh as usize - 2],
                            0,
                            0,
                            log2_cuw,
                            log2_cuh,
                            log2_cuw,
                            cud,
                            &self.core.tree_cons,
                        );
                        cost_best = cost_temp_dqp;
                        best_split_mode = SplitMode::NO_SPLIT;
                        s_temp_depth =
                            self.core.s_next_best[log2_cuw as usize - 2][log2_cuh as usize - 2];
                        dqp_temp_depth =
                            self.core.dqp_next_best[log2_cuw as usize - 2][log2_cuh as usize - 2];

                        if let Some(pic) = &self.pic[PIC_IDX_MODE] {
                            let cu_data_best = &mut self.core.cu_data_best[log2_cuw as usize - 2]
                                [log2_cuh as usize - 2];
                            cu_data_best.mode_cpy_rec_to_ref(
                                x0 as usize,
                                y0 as usize,
                                cuw as usize,
                                cuh as usize,
                                &mut pic.borrow().frame.borrow_mut().planes,
                                &self.core.tree_cons,
                            );
                        }

                        if evc_check_luma(&self.core.tree_cons) {
                            // update history MV list
                            // in mode_coding_unit, self.fn_pinter_analyze_cu will store the best MV in mi
                            // if the cost_temp has been update above, the best MV is in mi
                            self.mode.get_cu_pred_data(
                                &self.core.cu_data_best[log2_cuw as usize - 2]
                                    [log2_cuh as usize - 2],
                                0,
                                0,
                                log2_cuw,
                                log2_cuh,
                                log2_cuw,
                                cud,
                            );
                        }
                    }
                }
                if is_dqp_set && self.core.cu_qp_delta_code_mode == 2 {
                    self.core.cu_qp_delta_code_mode = 0;
                }
                cost_temp = cost_best;
                self.core.cu_mode = cu_mode_dqp;
                self.core.dist_cu_best = dist_cu_best_dqp;

            /*#if TRACE_COSTS
                        EVC_TRACE_COUNTER;
                        EVC_TRACE_STR("Block [");
                        EVC_TRACE_INT(x0);
                        EVC_TRACE_STR(", ");
                        EVC_TRACE_INT(y0);
                        EVC_TRACE_STR("]x(");
                        EVC_TRACE_INT(cuw);
                        EVC_TRACE_STR("x");
                        EVC_TRACE_INT(cuh);
                        EVC_TRACE_STR(") split_type ");
                        EVC_TRACE_INT(NO_SPLIT);
                        EVC_TRACE_STR(" cost is ");
                        EVC_TRACE_DOUBLE(cost_temp);
                        EVC_TRACE_STR("\n");
            #endif*/
            } else {
                cost_temp = MAX_COST;
            }
        }
    }

    fn check_nev_block(
        &mut self,
        x0: u16,
        y0: u16,
        log2_cuw: u8,
        log2_cuh: u8,
        do_curr: &mut bool,
        do_split: &mut bool,
        cud: u16,
        nbr_map_skip_flag: &mut bool,
    ) -> i32 {
        let mut nbr_map_skipcnt = 0;
        let mut nbr_map_cnt = 0;

        let x_scu = (x0 >> MIN_CU_LOG2);
        let y_scu = (y0 >> MIN_CU_LOG2);

        let cup = y_scu as u32 * self.w_scu as u32 + x_scu as u32;

        let log2_scuw = log2_cuw - MIN_CU_LOG2 as u8;
        let log2_scuh = log2_cuh - MIN_CU_LOG2 as u8;
        let scuw = 1 << log2_scuw;
        let scuh = 1 << log2_scuh;

        let mut size_cnt = vec![0; MAX_CU_DEPTH];

        *do_curr = true;
        *do_split = true;
        let avail_cu = evc_get_avail_block(
            x_scu,
            y_scu,
            self.w_scu,
            self.h_scu,
            cup,
            log2_cuw,
            log2_cuh,
            &self.map_scu,
        );

        let mut min_depth = MAX_CU_DEPTH as i8;
        let mut max_depth = 0;

        if IS_AVAIL(avail_cu, AVAIL_UP) {
            for w in 0..scuw {
                let pos = (cup - self.w_scu as u32 + w) as usize;

                let tmp = self.map_depth[pos];
                min_depth = if tmp < min_depth { tmp } else { min_depth };
                max_depth = if tmp > max_depth { tmp } else { max_depth };

                nbr_map_skipcnt += if self.map_scu[pos].GET_SF() != 0 {
                    1
                } else {
                    0
                };
                nbr_map_cnt += 1;
            }
        }

        if IS_AVAIL(avail_cu, AVAIL_UP_RI) {
            let pos = (cup - self.w_scu as u32 + scuw) as usize;

            let tmp = self.map_depth[pos];
            min_depth = if tmp < min_depth { tmp } else { min_depth };
            max_depth = if tmp > max_depth { tmp } else { max_depth };
        }

        if IS_AVAIL(avail_cu, AVAIL_LE) {
            for h in 0..scuh {
                let pos = (cup - 1 + (h * self.w_scu) as u32) as usize;

                let tmp = self.map_depth[pos];
                min_depth = if tmp < min_depth { tmp } else { min_depth };
                max_depth = if tmp > max_depth { tmp } else { max_depth };

                nbr_map_skipcnt += if self.map_scu[pos].GET_SF() != 0 {
                    1
                } else {
                    0
                };
                nbr_map_cnt += 1;
            }
        }

        if IS_AVAIL(avail_cu, AVAIL_LO_LE) {
            let pos = (cup + (self.w_scu * scuh) as u32 - 1) as usize;

            let tmp = self.map_depth[pos];
            min_depth = if tmp < min_depth { tmp } else { min_depth };
            max_depth = if tmp > max_depth { tmp } else { max_depth };
        }

        if IS_AVAIL(avail_cu, AVAIL_UP_LE) {
            let pos = (cup - self.w_scu as u32 - 1) as usize;

            let tmp = self.map_depth[pos];
            min_depth = if tmp < min_depth { tmp } else { min_depth };
            max_depth = if tmp > max_depth { tmp } else { max_depth };
        }

        if IS_AVAIL(avail_cu, AVAIL_RI) {
            for h in 0..scuh {
                let pos = (cup + scuw + (h * self.w_scu) as u32) as usize;

                let tmp = self.map_depth[pos];
                min_depth = if tmp < min_depth { tmp } else { min_depth };
                max_depth = if tmp > max_depth { tmp } else { max_depth };

                nbr_map_skipcnt += if self.map_scu[pos].GET_SF() != 0 {
                    1
                } else {
                    0
                };
                nbr_map_cnt += 1;
            }
        }

        if IS_AVAIL(avail_cu, AVAIL_LO_RI) {
            let pos = (cup + (self.w_scu * scuh) as u32 + scuw) as usize;

            let tmp = self.map_depth[pos];
            min_depth = if tmp < min_depth { tmp } else { min_depth };
            max_depth = if tmp > max_depth { tmp } else { max_depth };
        }

        if avail_cu != 0 {
            if cud < (min_depth - 1) as u16 {
                if log2_cuw > MIN_CU_LOG2 as u8 && log2_cuh > MIN_CU_LOG2 as u8 {
                    *do_curr = false;
                } else {
                    *do_curr = true;
                }
            }

            if cud > (max_depth + 1) as u16 {
                *do_split = if *do_curr { false } else { true };
            }
        } else {
            max_depth = MAX_CU_DEPTH as i8;
            min_depth = 0;
        }

        *nbr_map_skip_flag = false;
        if self.slice_type != SliceType::EVC_ST_I && nbr_map_skipcnt > (nbr_map_cnt / 2) {
            *nbr_map_skip_flag = true;
        }

        return max_depth as i32;
    }

    fn check_run_split(
        &mut self,
        log2_cuw: u8,
        log2_cuh: u8,
        cup: u16,
        next_split: bool,
        do_curr: bool,
        do_split: bool,
        split_allow: &mut [bool],
        boundary: bool,
        tree_cons: &TREE_CONS,
    ) {
        let min_cost = MAX_COST;
        let mut run_list = vec![false; MAX_SPLIT_NUM]; //a smaller set of allowed split modes based on a save & load technique

        if !next_split {
            split_allow[0] = true;

            for i in 1..MAX_SPLIT_NUM {
                split_allow[i] = false;
            }

            return;
        }

        for i in 0..MAX_SPLIT_NUM {
            run_list[i] = true;
        }

        run_list[0] = run_list[0] && do_curr;
        for i in 1..MAX_SPLIT_NUM {
            run_list[i] = run_list[i] && do_split;
        }

        //modified split_allow by the save & load decision
        let mut num_run = 0;
        split_allow[0] = run_list[0];
        for i in 1..MAX_SPLIT_NUM {
            split_allow[i] = run_list[i] && split_allow[i];
            num_run += if split_allow[i] { 1 } else { 0 };
        }

        //if all further splitting modes are not tried, at least we need try NO_SPLIT
        if num_run == 0 {
            split_allow[0] = true;
        }
    }

    fn get_min_max_qp(
        &mut self,
        min_qp: &mut u8,
        max_qp: &mut u8,
        is_dqp_set: &mut bool,
        split_mode: SplitMode,
        cuw: u16,
        cuh: u16,
        qp: u8,
        x0: u16,
        y0: u16,
    ) {
        *is_dqp_set = false;
        if !self.pps.cu_qp_delta_enabled_flag {
            *min_qp = self.qp; // Clip?
            *max_qp = self.qp;
        } else {
            if !self.sps.dquant_flag {
                if split_mode != SplitMode::NO_SPLIT {
                    *min_qp = qp; // Clip?
                    *max_qp = qp;
                } else {
                    *min_qp = self.qp;
                    *max_qp = self.qp + self.sh.dqp;
                }
            } else {
                *min_qp = qp; // Clip?
                *max_qp = qp;
                if split_mode == SplitMode::NO_SPLIT
                    && CONV_LOG2(cuw as usize) + CONV_LOG2(cuh as usize)
                        >= self.pps.cu_qp_delta_area
                    && self.core.cu_qp_delta_code_mode != 2
                {
                    self.core.cu_qp_delta_code_mode = 1;
                    *min_qp = self.qp;
                    *max_qp = self.qp + self.sh.dqp;

                    if CONV_LOG2(cuw as usize) == 7 || CONV_LOG2(cuh as usize) == 7 {
                        *is_dqp_set = true;
                        self.core.cu_qp_delta_code_mode = 2;
                    } else {
                        *is_dqp_set = false;
                    }
                } else if (CONV_LOG2(cuw as usize) + CONV_LOG2(cuh as usize)
                    == self.pps.cu_qp_delta_area + 1)
                    || (CONV_LOG2(cuh as usize) + CONV_LOG2(cuw as usize)
                        == self.pps.cu_qp_delta_area
                        && self.core.cu_qp_delta_code_mode != 2)
                {
                    self.core.cu_qp_delta_code_mode = 2;
                    *is_dqp_set = true;
                    *min_qp = self.qp;
                    *max_qp = self.qp + self.sh.dqp;
                }
            }
        }
    }

    fn clear_map_scu(&mut self, x: u16, y: u16, mut cuw: u16, mut cuh: u16) {
        let map_cu_mode = &mut self.map_cu_mode
            [((y >> MIN_CU_LOG2) * self.w_scu + (x >> MIN_CU_LOG2)) as usize..];
        let map_scu =
            &mut self.map_scu[((y >> MIN_CU_LOG2) * self.w_scu + (x >> MIN_CU_LOG2)) as usize..];

        if x + cuw > self.w {
            cuw = self.w - x;
        }

        if y + cuh > self.h {
            cuh = self.h - y;
        }

        let w = (cuw >> MIN_CU_LOG2) as usize;
        let h = (cuh >> MIN_CU_LOG2) as usize;

        for j in 0..h {
            for i in 0..w {
                map_scu[j * self.w_scu as usize + i] = MCU::default();
                map_cu_mode[j * self.w_scu as usize + i] = MCU::default();
            }
        }
    }
}