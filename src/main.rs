use nonprofit_balance_guard::run_service;

fn main() {
    match run_service() {
        Ok(report) => println!(
            "campaign={} receipts={} reminders={} recharge_needed={}",
            report.campaign, report.receipts, report.reminders, report.recharge_needed
        ),
        Err(error) => eprintln!("service failed: {error:?}"),
    }
}
