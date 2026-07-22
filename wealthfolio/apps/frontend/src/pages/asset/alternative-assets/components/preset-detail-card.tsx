import { format, differenceInDays } from "date-fns";
import { Card, CardContent, CardHeader, CardTitle } from "@wealthfolio/ui/components/ui/card";
import { Badge } from "@wealthfolio/ui/components/ui/badge";
import { Progress } from "@wealthfolio/ui/components/ui/progress";
import { Icons } from "@wealthfolio/ui/components/ui/icons";
import { AmountDisplay } from "@wealthfolio/ui";

interface PresetDetailCardProps {
  preset: string;
  metadata: Record<string, unknown>;
  currency: string;
}

export function PresetDetailCard({ preset, metadata, currency }: PresetDetailCardProps) {
  // Use presetFields from metadata if available, otherwise just use metadata
  const data = (metadata?.presetFields as Record<string, any>) || metadata || {};
  
  const formatDate = (dateStr?: string) => {
    if (!dateStr) return "-";
    try {
      return format(new Date(dateStr), "MMM d, yyyy");
    } catch {
      return dateStr;
    }
  };

  const calculateDaysRemaining = (endDateStr?: string) => {
    if (!endDateStr) return null;
    try {
      const end = new Date(endDateStr);
      const now = new Date();
      const diff = differenceInDays(end, now);
      return diff;
    } catch {
      return null;
    }
  };

  const renderField = (label: string, value: React.ReactNode) => (
    <div className="space-y-1">
      <p className="text-muted-foreground text-xs font-medium">{label}</p>
      <p className="text-sm font-medium">{value || "-"}</p>
    </div>
  );

  const renderMoney = (amount?: string | number) => {
    if (amount === undefined || amount === null) return "-";
    const num = typeof amount === "string" ? parseFloat(amount) : amount;
    if (isNaN(num)) return "-";
    return <AmountDisplay value={num} currency={currency} />;
  };

  const renderProgress = (startDate?: string, endDate?: string) => {
    if (!startDate || !endDate) return null;
    try {
      const start = new Date(startDate);
      const end = new Date(endDate);
      const now = new Date();
      
      const total = end.getTime() - start.getTime();
      const elapsed = now.getTime() - start.getTime();
      
      if (total <= 0) return null;
      
      const percent = Math.max(0, Math.min(100, (elapsed / total) * 100));
      const daysLeft = differenceInDays(end, now);
      
      return (
        <div className="space-y-2 mt-4">
          <div className="flex justify-between text-xs">
            <span className="text-muted-foreground">Term Progress</span>
            <span className="font-medium">{daysLeft > 0 ? `${daysLeft} days remaining` : "Completed"}</span>
          </div>
          <Progress value={percent} className="h-2" />
          <div className="flex justify-between text-xs text-muted-foreground">
            <span>{formatDate(startDate)}</span>
            <span>{formatDate(endDate)}</span>
          </div>
        </div>
      );
    } catch {
      return null;
    }
  };

  const renderFixedDeposit = () => {
    const principal = Number(data.principal || 0);
    const currentValue = Number(data.currentValue || principal);
    const accruedInterest = currentValue - principal;
    
    return (
      <div className="space-y-4">
        <div className="flex justify-between items-start">
          <div>
            <h4 className="font-semibold text-lg">{data.bank_name || "Fixed Deposit"}</h4>
            {data.account_number && <p className="text-muted-foreground text-sm">A/C: {data.account_number}</p>}
          </div>
          {data.tax_saver && <Badge variant="secondary" className="bg-green-500/10 text-green-500 hover:bg-green-500/20">Tax Saver</Badge>}
        </div>
        
        <div className="grid grid-cols-2 gap-4 mt-4">
          {renderField("Principal Amount", renderMoney(data.principal))}
          {renderField("Interest Rate", `${data.interest_rate_pct || "-"}% ${data.compounding_frequency ? `(${data.compounding_frequency})` : ""}`)}
          {renderField("Accrued Interest", accruedInterest > 0 ? renderMoney(accruedInterest) : "-")}
          {renderField("Maturity Value", renderMoney(data.maturity_value))}
        </div>
        
        {renderProgress(data.start_date, data.maturity_date)}
      </div>
    );
  };

  const renderRecurringDeposit = () => {
    return (
      <div className="space-y-4">
        <div className="flex justify-between items-start">
          <div>
            <h4 className="font-semibold text-lg">{data.bank_name || "Recurring Deposit"}</h4>
            {data.account_number && <p className="text-muted-foreground text-sm">A/C: {data.account_number}</p>}
          </div>
        </div>
        
        <div className="grid grid-cols-2 gap-4 mt-4">
          {renderField("Monthly Installment", renderMoney(data.monthly_installment))}
          {renderField("Interest Rate", `${data.interest_rate_pct || "-"}%`)}
          {renderField("Installments Paid", data.installments_paid ? `${data.installments_paid} / ${data.total_installments || "-"}` : "-")}
          {renderField("Maturity Value", renderMoney(data.maturity_value))}
        </div>
        
        {renderProgress(data.start_date, data.maturity_date)}
      </div>
    );
  };

  const renderPPF = () => {
    return (
      <div className="space-y-4">
        <div className="flex justify-between items-start">
          <div>
            <h4 className="font-semibold text-lg">{data.bank_name || "PPF Account"}</h4>
            {data.account_number && <p className="text-muted-foreground text-sm">A/C: {data.account_number}</p>}
          </div>
          <Badge variant="outline">15-Yr Lock-in</Badge>
        </div>
        
        <div className="grid grid-cols-2 gap-4 mt-4">
          {renderField("Current Balance", renderMoney(data.current_balance || data.currentValue))}
          {renderField("Interest Rate", `${data.interest_rate_pct || "-"}%`)}
          {renderField("Opening Date", formatDate(data.opening_date))}
          {renderField("FY Contribution", renderMoney(data.fy_contribution))}
        </div>
      </div>
    );
  };

  const renderEPF = () => {
    return (
      <div className="space-y-4">
        <div className="flex justify-between items-start">
          <div>
            <h4 className="font-semibold text-lg">{data.employer_name || "EPF Account"}</h4>
            {data.uan && <p className="text-muted-foreground text-sm">UAN: {data.uan}</p>}
          </div>
        </div>
        
        <div className="grid grid-cols-2 gap-4 mt-4">
          {renderField("Current Balance", renderMoney(data.current_balance || data.currentValue))}
          {renderField("Employee Contribution", renderMoney(data.employee_contribution))}
          {renderField("Employer Contribution", renderMoney(data.employer_contribution))}
          {renderField("Last Updated", formatDate(data.last_updated_date))}
        </div>
      </div>
    );
  };

  const renderNPS = () => {
    return (
      <div className="space-y-4">
        <div className="flex justify-between items-start">
          <div>
            <h4 className="font-semibold text-lg">NPS Account</h4>
            {data.pran && <p className="text-muted-foreground text-sm">PRAN: {data.pran}</p>}
          </div>
          <Badge variant="secondary" className={data.tier === "II" ? "bg-blue-500/10 text-blue-500" : "bg-orange-500/10 text-orange-500"}>
            Tier {data.tier || "I"} {data.tier === "II" ? "(Liquid)" : "(Locked)"}
          </Badge>
        </div>
        
        <div className="grid grid-cols-2 gap-4 mt-4">
          {renderField("Fund Manager", data.fund_manager)}
          {renderField("Scheme Preference", data.scheme_preference)}
          {renderField("Units Held", data.units_held)}
          {renderField("NAV", renderMoney(data.nav))}
        </div>
      </div>
    );
  };

  const renderInsurance = () => {
    let typeLabel = "Insurance";
    if (preset === "insurance_term") typeLabel = "Term Insurance";
    else if (preset === "insurance_endowment") typeLabel = "Endowment Plan";
    else if (preset === "insurance_ulip") typeLabel = "ULIP";
    else if (preset === "insurance_health") typeLabel = "Health Insurance";

    const daysToPremium = calculateDaysRemaining(data.premium_due_date);
    let premiumStatusColor = "text-muted-foreground";
    let premiumStatusText = "Unknown";
    
    if (daysToPremium !== null) {
      if (daysToPremium < 0) {
        premiumStatusColor = "text-red-500";
        premiumStatusText = `Overdue by ${Math.abs(daysToPremium)} days`;
      } else if (daysToPremium <= 30) {
        premiumStatusColor = "text-yellow-500";
        premiumStatusText = `Upcoming in ${daysToPremium} days`;
      } else {
        premiumStatusColor = "text-green-500";
        premiumStatusText = `Due in ${daysToPremium} days`;
      }
    }

    return (
      <div className="space-y-4">
        <div className="flex justify-between items-start">
          <div>
            <h4 className="font-semibold text-lg">{data.insurer || "Insurer"}</h4>
            {data.policy_number && <p className="text-muted-foreground text-sm">Policy: {data.policy_number}</p>}
          </div>
          <Badge variant="outline">{typeLabel}</Badge>
        </div>
        
        <div className="grid grid-cols-2 gap-4 mt-4">
          {renderField("Sum Assured", renderMoney(data.sum_assured))}
          {renderField(
            "Premium", 
            <div className="flex flex-col">
              <span>{renderMoney(data.premium_amount)}</span>
              {data.premium_due_date && <span className={`text-xs ${premiumStatusColor}`}>{premiumStatusText}</span>}
            </div>
          )}
          
          {preset === "insurance_endowment" && (
            <>
              {renderField("Surrender Value", renderMoney(data.surrender_value))}
              {renderField("Maturity Value", renderMoney(data.maturity_value))}
            </>
          )}
          
          {preset === "insurance_ulip" && (
            <>
              {renderField("Fund Value", renderMoney(data.fund_value || data.currentValue))}
              {renderField("NAV / Units", `${renderMoney(data.nav)} / ${data.units || "-"}`)}
            </>
          )}
          
          {preset === "insurance_health" && (
            <>
              {renderField("Covered Members", data.family_members || "-")}
            </>
          )}
        </div>
      </div>
    );
  };

  const renderLoan = () => {
    let typeLabel = "Loan";
    if (preset === "home_loan") typeLabel = "Home Loan";
    else if (preset === "car_loan") typeLabel = "Car Loan";
    else if (preset === "personal_loan") typeLabel = "Personal Loan";

    const principal = Number(data.principal || 0);
    const outstanding = Number(data.outstanding_balance || 0);
    const paid = principal - outstanding;
    const progressPercent = principal > 0 ? Math.max(0, Math.min(100, (paid / principal) * 100)) : 0;

    return (
      <div className="space-y-4">
        <div className="flex justify-between items-start">
          <div>
            <h4 className="font-semibold text-lg">{data.lender || "Lender"}</h4>
            {data.loan_account_number && <p className="text-muted-foreground text-sm">A/C: {data.loan_account_number}</p>}
          </div>
          <Badge variant="outline">{typeLabel}</Badge>
        </div>
        
        <div className="grid grid-cols-2 gap-4 mt-4">
          {renderField("Principal", renderMoney(data.principal))}
          {renderField("Interest Rate", `${data.interest_rate_pct || "-"}%`)}
          {renderField("EMI Amount", renderMoney(data.emi_amount))}
          {renderField("Outstanding Balance", renderMoney(outstanding))}
          {renderField("Tenure / EMIs Left", `${data.tenure_months ? data.tenure_months + ' mos' : '-'} / ${data.emis_remaining || '-'}`)}
          {data.linked_asset_id && renderField("Linked Asset", <span className="flex items-center gap-1 text-primary"><Icons.Link className="h-3 w-3"/> Linked</span>)}
        </div>
        
        <div className="space-y-2 mt-4">
          <div className="flex justify-between text-xs">
            <span className="text-muted-foreground">Repayment Progress</span>
            <span className="font-medium">{progressPercent.toFixed(1)}%</span>
          </div>
          <Progress value={progressPercent} className="h-2" />
        </div>
      </div>
    );
  };

  const renderCreditCard = () => {
    const limit = Number(data.credit_limit || 0);
    const outstanding = Number(data.current_outstanding || 0);
    const available = Number(data.available_limit || (limit - outstanding));
    const utilization = limit > 0 ? (outstanding / limit) * 100 : 0;
    
    let utilColor = "text-green-500";
    let utilBg = "bg-green-500";
    if (utilization > 60) {
      utilColor = "text-red-500";
      utilBg = "bg-red-500";
    } else if (utilization > 30) {
      utilColor = "text-yellow-500";
      utilBg = "bg-yellow-500";
    }
    
    const daysToPayment = calculateDaysRemaining(data.payment_due_date);

    return (
      <div className="space-y-4">
        <div className="flex justify-between items-start">
          <div>
            <h4 className="font-semibold text-lg">{data.issuer || "Credit Card"}</h4>
            {data.last_4_digits && <p className="text-muted-foreground text-sm">**** {data.last_4_digits}</p>}
          </div>
        </div>
        
        <div className="grid grid-cols-2 gap-4 mt-4">
          {renderField("Credit Limit", renderMoney(limit))}
          {renderField("Current Outstanding", <span className="text-red-500 font-medium">{renderMoney(outstanding)}</span>)}
          {renderField("Available Limit", renderMoney(available))}
          {renderField(
            "Payment Due", 
            <div className="flex flex-col">
              <span>{formatDate(data.payment_due_date)}</span>
              {daysToPayment !== null && <span className="text-xs text-muted-foreground">in {daysToPayment} days</span>}
            </div>
          )}
        </div>
        
        <div className="space-y-2 mt-4">
          <div className="flex justify-between text-xs">
            <span className="text-muted-foreground">Credit Utilization</span>
            <span className={`font-medium ${utilColor}`}>{utilization.toFixed(1)}%</span>
          </div>
          <Progress value={utilization} className={`h-2 ${utilBg}`} />
        </div>
      </div>
    );
  };

  const renderCashInHand = () => {
    return (
      <div className="space-y-4">
        <div className="flex justify-between items-start">
          <h4 className="font-semibold text-lg">Cash</h4>
        </div>
        <div className="grid grid-cols-2 gap-4 mt-4">
          {renderField("Amount", renderMoney(data.amount || data.currentValue))}
          {renderField("Location/Description", data.location || "-")}
        </div>
      </div>
    );
  };

  const renderContent = () => {
    switch (preset) {
      case "fixed_deposit": return renderFixedDeposit();
      case "recurring_deposit": return renderRecurringDeposit();
      case "ppf": return renderPPF();
      case "epf": return renderEPF();
      case "nps": return renderNPS();
      case "insurance_term":
      case "insurance_endowment":
      case "insurance_ulip":
      case "insurance_health": return renderInsurance();
      case "home_loan":
      case "car_loan":
      case "personal_loan": return renderLoan();
      case "credit_card": return renderCreditCard();
      case "cash_in_hand": return renderCashInHand();
      default: return null;
    }
  };

  const content = renderContent();
  if (!content) return null;

  return (
    <Card className="overflow-hidden">
      <CardHeader className="bg-muted/30 pb-4">
        <CardTitle className="text-sm font-medium flex items-center gap-2">
          <Icons.FileText className="h-4 w-4" />
          Instrument Details
        </CardTitle>
      </CardHeader>
      <CardContent className="pt-6">
        {content}
      </CardContent>
    </Card>
  );
}
