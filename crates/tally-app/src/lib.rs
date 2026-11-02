pub mod accounts;
pub mod bootstrap;
pub mod cards;
pub mod categories;
pub mod dto;
pub mod error;
pub mod movements;
pub mod ports;
mod validation;

pub use accounts::{AccountsRepository, AccountsService, NewAccount, OpeningBalance};
pub use bootstrap::BootstrapService;
pub use cards::{
    CardsRepository, CardsService, NewCardPayment, NewCardPurchase, NewInstallment,
    NewRecognitionEntry,
};
pub use categories::{CategoriesRepository, CategoriesService, NewCategory};
pub use dto::{
    AccountDto, AccountStatementDto, BootstrapDto, CardPaymentDto, CardSummaryDto, CategoryDto,
    CreateAccountInput, CreateAdjustmentInput, CreateCardPaymentInput, CreateCardPurchaseInput,
    CreateCategoryInput, CreateExpenseInput, CreateIncomeInput, CreateTransferInput,
    InstallmentAllocationDto, InstallmentDto, InstallmentPlanDto, IntegrityReportDto, MovementDto,
    MovementFilter, SettingsDto, StatementLineDto, UpdateMovementInput,
};
pub use error::{AppError, AppErrorDto};
pub use movements::{MovementsRepository, MovementsService, NewMovement};
pub use ports::BootstrapRepository;
