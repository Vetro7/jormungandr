use super::logs::internal::Logs;
use super::pool::internal::Pool;
use crate::{
    blockcfg::{BlockDate, Contents, ContentsBuilder, Ledger, LedgerParameters},
    fragment::FragmentId,
};
use chain_core::property::Fragment as _;
use jormungandr_lib::interfaces::FragmentStatus;

pub enum SelectionOutput {
    Commit { fragment_id: FragmentId },
    RequestSmallerFee,
    RequestSmallerSize,
    Reject { reason: String },
}

pub trait FragmentSelectionAlgorithm {
    fn select(
        &mut self,
        ledger: &Ledger,
        ledger_params: &LedgerParameters,
        block_date: BlockDate,
        logs: &mut Logs,
        pool: &mut Pool,
    );

    fn finalize(self) -> Contents;
}

pub struct OldestFirst {
    builder: ContentsBuilder,
    max_per_block: usize,
}

impl OldestFirst {
    pub fn new(max_per_block: usize) -> Self {
        OldestFirst {
            builder: ContentsBuilder::new(),
            max_per_block,
        }
    }
}

impl FragmentSelectionAlgorithm for OldestFirst {
    fn finalize(self) -> Contents {
        self.builder.into()
    }

    fn select(
        &mut self,
        ledger: &Ledger,
        ledger_params: &LedgerParameters,
        block_date: BlockDate,
        logs: &mut Logs,
        pool: &mut Pool,
    ) {
        let mut total = 0usize;
        let mut ledger_simulation = ledger.clone();

        while let Some(fragment) = pool.remove_oldest() {
            let id = fragment.id();
            match ledger_simulation.apply_fragment(ledger_params, &fragment, block_date) {
                Ok(ledger_new) => {
                    self.builder.push(fragment);
                    total += 1;
                    ledger_simulation = ledger_new;
                }
                Err(error) => {
                    use std::error::Error as _;
                    let error = if let Some(source) = error.source() {
                        format!("{}: {}", error, source)
                    } else {
                        error.to_string()
                    };
                    logs.modify(&id.into(), FragmentStatus::Rejected { reason: error })
                }
            }
            if total >= self.max_per_block {
                break;
            }
        }
    }
}
