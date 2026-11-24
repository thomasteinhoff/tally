pub mod accounts;
pub mod bootstrap;
pub mod cards;
pub mod categories;
pub mod csv;
pub mod dto;
pub mod error;
pub mod movements;
pub mod ports;
pub mod reports;
pub mod savings;
mod validation;

pub use accounts::{AccountsRepository, AccountsService, NewAccount, OpeningBalance};
pub use bootstrap::BootstrapService;
pub use cards::{
    CardsRepository, CardsService, NewCardPayment, NewCardPurchase, NewInstallment,
    NewRecognitionEntry,
};
pub use categories::{CategoriesRepository, CategoriesService, NewCategory};
pub use csv::movements_csv;
pub use dto::{
    AccountDto, AccountStatementDto, BackupDto, BootstrapDto, CardPaymentDto, CardSummaryDto,
    CashFlowGroupDto, CashFlowMonthDto, CashFlowReportDto, CategoryDto, CategoryReportDto,
    CategoryTotalDto, CommitmentDto, CommitmentMonthDto, CommitmentsReportDto, CreateAccountInput,
    CreateAdjustmentInput, CreateCardPaymentInput, CreateCardPurchaseInput, CreateCategoryInput,
    CreateExpenseInput, CreateIncomeInput, CreateSavingsYieldInput, CreateTransferInput,
    CsvExportDto, DashboardDto, ExpenseTrendDto, ExportCsvInput, InstallmentAllocationDto,
    InstallmentDto, InstallmentPlanDto, IntegrityReportDto, MonthlyExpenseDto, MovementDto,
    MovementFilter, NetWorthAccountDto, NetWorthHistoryDto, NetWorthPointDto, NetWorthReportDto,
    PeriodSpendingBucketDto, PeriodSpendingReportDto, ReplaceSavingsYieldInput, ReportFilter,
    RestoreResultDto, ReverseSavingsYieldInput, SavingsOverviewDto, SavingsYieldDto, SettingsDto,
    StatementLineDto, UpdateInstallmentPlanInput, UpdateMovementInput, UpdateSettingsInput,
};
pub use error::{AppError, AppErrorDto};
pub use movements::{MovementsRepository, MovementsService, NewMovement};
pub use ports::BootstrapRepository;
pub use reports::{ReportQuery, ReportsRepository, ReportsService};
pub use savings::{
    NewSavingsReversal, NewSavingsYield, SavingsConfig, SavingsRepository, SavingsService,
    YieldCalculation, YieldMode, calculate_yield, daily_rate, normalize_savings_config,
};
