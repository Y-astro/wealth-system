import { Label } from "@wealthfolio/ui/components/ui/label";
import { Input } from "@wealthfolio/ui/components/ui/input";
import { Checkbox } from "@wealthfolio/ui/components/ui/checkbox";
import { DatePickerInput, ResponsiveSelect, MoneyInput } from "@wealthfolio/ui";
import { INSTRUMENT_PRESETS } from "./alternative-asset-quick-add-schema";

export function PresetFormFields({ presetCategory, instrumentPreset, formData, updateFormData }: any) {
  const presets = INSTRUMENT_PRESETS.filter((p) => p.category === (presetCategory === "Indian Instruments" ? "indian" : "liability"));

  const handlePresetSelect = (val: string) => {
    updateFormData("instrumentPreset", val);
    updateFormData("presetFields", {});
    
    // Auto-fill some things
    const presetLabel = presets.find(p => p.value === val)?.label || "";
    if (presetLabel) {
      updateFormData("name", presetLabel);
    }
  };

  const updatePresetField = (field: string, val: any) => {
    updateFormData("presetFields", {
      ...(formData.presetFields || {}),
      [field]: val
    });
    
    // Auto-generate names based on fields
    const f = { ...formData.presetFields, [field]: val };
    if (instrumentPreset === "fixed_deposit" || instrumentPreset === "recurring_deposit") {
      updateFormData("name", `${f.bank_name || "Bank"} ${instrumentPreset === "fixed_deposit" ? "FD" : "RD"}`);
    } else if (instrumentPreset === "ppf") {
      updateFormData("name", `PPF - ${f.bank_name || "Bank/Post Office"}`);
    } else if (instrumentPreset === "epf") {
      updateFormData("name", `EPF - ${f.employer_name || f.uan || "Employer"}`);
    } else if (instrumentPreset === "nps") {
      updateFormData("name", `NPS Tier ${f.tier || "I"}`);
    } else if (instrumentPreset?.startsWith("insurance")) {
      updateFormData("name", `${f.insurer || "Insurer"} Insurance`);
    } else if (instrumentPreset?.endsWith("loan")) {
      updateFormData("name", `${f.lender || "Lender"} Loan`);
    } else if (instrumentPreset === "credit_card") {
      updateFormData("name", `${f.issuer || "Issuer"} Credit Card`);
    } else if (instrumentPreset === "cash_in_hand") {
      updateFormData("name", "Cash in Hand");
    }
  };

  const f = formData.presetFields || {};

  return (
    <div className="space-y-4">
      <div className="space-y-2">
        <Label className="text-foreground text-sm font-medium">Instrument Type</Label>
        <ResponsiveSelect
          value={instrumentPreset || ""}
          onValueChange={handlePresetSelect}
          options={presets}
          placeholder="Select instrument"
          sheetTitle="Select Instrument"
        />
      </div>

      {instrumentPreset && (
        <div className="space-y-4 border-t pt-4">
          {(instrumentPreset === "fixed_deposit" || instrumentPreset === "recurring_deposit") && (
            <>
              <div className="space-y-2">
                <Label className="text-sm font-medium">Bank Name</Label>
                <Input value={f.bank_name || ""} onChange={e => updatePresetField("bank_name", e.target.value)} />
              </div>
              {instrumentPreset === "recurring_deposit" ? (
                <div className="space-y-2">
                  <Label className="text-sm font-medium">Monthly Installment</Label>
                  <MoneyInput value={f.monthly_installment || ""} onValueChange={v => updatePresetField("monthly_installment", v)} />
                </div>
              ) : (
                <div className="space-y-2">
                  <Label className="text-sm font-medium">Principal Amount</Label>
                  <MoneyInput value={f.principal || ""} onValueChange={v => { updatePresetField("principal", v); updateFormData("currentValue", v); }} />
                </div>
              )}
              <div className="space-y-2">
                <Label className="text-sm font-medium">Interest Rate %</Label>
                <Input type="number" value={f.interest_rate_pct || ""} onChange={e => updatePresetField("interest_rate_pct", e.target.value)} />
              </div>
              {instrumentPreset === "fixed_deposit" && (
                <div className="space-y-2">
                  <Label className="text-sm font-medium">Compounding Frequency</Label>
                  <ResponsiveSelect
                    value={f.compounding_frequency || "quarterly"}
                    onValueChange={v => updatePresetField("compounding_frequency", v)}
                    options={[{value: "monthly", label: "Monthly"}, {value: "quarterly", label: "Quarterly"}, {value: "half_yearly", label: "Half-Yearly"}, {value: "yearly", label: "Yearly"}]}
                    placeholder="Select frequency"
                  />
                </div>
              )}
              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-2">
                  <Label className="text-sm font-medium">Start Date</Label>
                  <DatePickerInput value={f.start_date ? new Date(f.start_date) : undefined} onChange={v => updatePresetField("start_date", v?.toISOString())} />
                </div>
                <div className="space-y-2">
                  <Label className="text-sm font-medium">Maturity Date</Label>
                  <DatePickerInput value={f.maturity_date ? new Date(f.maturity_date) : undefined} onChange={v => updatePresetField("maturity_date", v?.toISOString())} />
                </div>
              </div>
              {instrumentPreset === "fixed_deposit" && (
                <>
                  <div className="space-y-2">
                    <Label className="text-sm font-medium">Account Number (Optional)</Label>
                    <Input value={f.account_number || ""} onChange={e => updatePresetField("account_number", e.target.value)} />
                  </div>
                  <div className="flex items-center space-x-2">
                    <Checkbox id="tax_saver" checked={f.tax_saver === "true"} onCheckedChange={c => updatePresetField("tax_saver", c ? "true" : "false")} />
                    <Label htmlFor="tax_saver">Tax Saver FD?</Label>
                  </div>
                </>
              )}
            </>
          )}

          {instrumentPreset === "ppf" && (
            <>
              <div className="space-y-2">
                <Label className="text-sm font-medium">Bank/Post Office</Label>
                <Input value={f.bank_name || ""} onChange={e => updatePresetField("bank_name", e.target.value)} />
              </div>
              <div className="space-y-2">
                <Label className="text-sm font-medium">Account Number</Label>
                <Input value={f.account_number || ""} onChange={e => updatePresetField("account_number", e.target.value)} />
              </div>
              <div className="space-y-2">
                <Label className="text-sm font-medium">Interest Rate % (Optional)</Label>
                <Input type="number" placeholder="7.1" value={f.interest_rate_pct || ""} onChange={e => updatePresetField("interest_rate_pct", e.target.value)} />
              </div>
              <div className="space-y-2">
                <Label className="text-sm font-medium">Opening Date (Optional)</Label>
                <DatePickerInput value={f.opening_date ? new Date(f.opening_date) : undefined} onChange={v => updatePresetField("opening_date", v?.toISOString())} />
              </div>
            </>
          )}

          {instrumentPreset === "epf" && (
            <>
              <div className="space-y-2">
                <Label className="text-sm font-medium">UAN Number</Label>
                <Input value={f.uan || ""} onChange={e => updatePresetField("uan", e.target.value)} />
              </div>
              <div className="space-y-2">
                <Label className="text-sm font-medium">Employer Name (Optional)</Label>
                <Input value={f.employer_name || ""} onChange={e => updatePresetField("employer_name", e.target.value)} />
              </div>
            </>
          )}

          {instrumentPreset === "nps" && (
            <>
              <div className="space-y-2">
                <Label className="text-sm font-medium">PRAN Number</Label>
                <Input value={f.pran || ""} onChange={e => updatePresetField("pran", e.target.value)} />
              </div>
              <div className="space-y-2">
                <Label className="text-sm font-medium">Tier</Label>
                <ResponsiveSelect
                  value={f.tier || "I"}
                  onValueChange={v => updatePresetField("tier", v)}
                  options={[{value: "I", label: "Tier I"}, {value: "II", label: "Tier II"}]}
                  placeholder="Select tier"
                />
              </div>
              <div className="space-y-2">
                <Label className="text-sm font-medium">Fund Manager (Optional)</Label>
                <Input value={f.fund_manager || ""} onChange={e => updatePresetField("fund_manager", e.target.value)} />
              </div>
              <div className="space-y-2">
                <Label className="text-sm font-medium">Scheme Preference (Optional)</Label>
                <ResponsiveSelect
                  value={f.scheme_preference || ""}
                  onValueChange={v => updatePresetField("scheme_preference", v)}
                  options={[{value: "E", label: "Equity (E)"}, {value: "C", label: "Corporate Debt (C)"}, {value: "G", label: "Government Securities (G)"}, {value: "A", label: "Alternative (A)"}]}
                  placeholder="Select scheme"
                />
              </div>
              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-2">
                  <Label className="text-sm font-medium">NAV (Optional)</Label>
                  <Input type="number" value={f.nav || ""} onChange={e => updatePresetField("nav", e.target.value)} />
                </div>
                <div className="space-y-2">
                  <Label className="text-sm font-medium">Units Held (Optional)</Label>
                  <Input type="number" value={f.units_held || ""} onChange={e => updatePresetField("units_held", e.target.value)} />
                </div>
              </div>
            </>
          )}

          {instrumentPreset?.startsWith("insurance") && (
            <>
              <div className="space-y-2">
                <Label className="text-sm font-medium">Policy Number</Label>
                <Input value={f.policy_number || ""} onChange={e => updatePresetField("policy_number", e.target.value)} />
              </div>
              <div className="space-y-2">
                <Label className="text-sm font-medium">Insurer</Label>
                <Input value={f.insurer || ""} onChange={e => updatePresetField("insurer", e.target.value)} />
              </div>
              <div className="space-y-2">
                <Label className="text-sm font-medium">Sum Assured/Insured (Optional)</Label>
                <MoneyInput value={f.sum_assured || ""} onValueChange={v => updatePresetField("sum_assured", v)} />
              </div>
              <div className="space-y-2">
                <Label className="text-sm font-medium">Premium Annual (Optional)</Label>
                <MoneyInput value={f.premium_annual || ""} onValueChange={v => updatePresetField("premium_annual", v)} />
              </div>
              <div className="space-y-2">
                <Label className="text-sm font-medium">Premium Due Date (Optional)</Label>
                <DatePickerInput value={f.premium_due_date ? new Date(f.premium_due_date) : undefined} onChange={v => updatePresetField("premium_due_date", v?.toISOString())} />
              </div>

              {instrumentPreset === "insurance_endowment" && (
                <div className="grid grid-cols-2 gap-4">
                  <div className="space-y-2">
                    <Label className="text-sm font-medium">Surrender Value</Label>
                    <MoneyInput value={f.surrender_value || ""} onValueChange={v => updatePresetField("surrender_value", v)} />
                  </div>
                  <div className="space-y-2">
                    <Label className="text-sm font-medium">Maturity Value</Label>
                    <MoneyInput value={f.maturity_value || ""} onValueChange={v => updatePresetField("maturity_value", v)} />
                  </div>
                </div>
              )}

              {instrumentPreset === "insurance_ulip" && (
                <div className="grid grid-cols-2 gap-4">
                  <div className="space-y-2">
                    <Label className="text-sm font-medium">NAV</Label>
                    <Input type="number" value={f.nav || ""} onChange={e => updatePresetField("nav", e.target.value)} />
                  </div>
                  <div className="space-y-2">
                    <Label className="text-sm font-medium">Units Held</Label>
                    <Input type="number" value={f.units_held || ""} onChange={e => updatePresetField("units_held", e.target.value)} />
                  </div>
                </div>
              )}
              {instrumentPreset === "insurance_health" && (
                <div className="space-y-2">
                  <Label className="text-sm font-medium">Family Members (Optional)</Label>
                  <Input value={f.family_members || ""} onChange={e => updatePresetField("family_members", e.target.value)} />
                </div>
              )}
            </>
          )}

          {instrumentPreset?.endsWith("loan") && (
            <>
              <div className="space-y-2">
                <Label className="text-sm font-medium">Lender</Label>
                <Input value={f.lender || ""} onChange={e => updatePresetField("lender", e.target.value)} />
              </div>
              <div className="space-y-2">
                <Label className="text-sm font-medium">Principal Amount</Label>
                <MoneyInput value={f.principal_amount || ""} onValueChange={v => { updatePresetField("principal_amount", v); }} />
              </div>
              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-2">
                  <Label className="text-sm font-medium">Interest Rate %</Label>
                  <Input type="number" value={f.interest_rate_pct || ""} onChange={e => updatePresetField("interest_rate_pct", e.target.value)} />
                </div>
                <div className="space-y-2">
                  <Label className="text-sm font-medium">Tenure Months</Label>
                  <Input type="number" value={f.tenure_months || ""} onChange={e => updatePresetField("tenure_months", e.target.value)} />
                </div>
              </div>
              <div className="space-y-2">
                <Label className="text-sm font-medium">EMI Amount</Label>
                <MoneyInput value={f.emi_amount || ""} onValueChange={v => updatePresetField("emi_amount", v)} />
              </div>
              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-2">
                  <Label className="text-sm font-medium">Disbursement Date</Label>
                  <DatePickerInput value={f.disbursement_date ? new Date(f.disbursement_date) : undefined} onChange={v => updatePresetField("disbursement_date", v?.toISOString())} />
                </div>
                <div className="space-y-2">
                  <Label className="text-sm font-medium">EMI Start Date (Optional)</Label>
                  <DatePickerInput value={f.emi_start_date ? new Date(f.emi_start_date) : undefined} onChange={v => updatePresetField("emi_start_date", v?.toISOString())} />
                </div>
              </div>
            </>
          )}

          {instrumentPreset === "credit_card" && (
            <>
              <div className="space-y-2">
                <Label className="text-sm font-medium">Card Issuer</Label>
                <Input value={f.issuer || ""} onChange={e => updatePresetField("issuer", e.target.value)} />
              </div>
              <div className="space-y-2">
                <Label className="text-sm font-medium">Credit Limit</Label>
                <MoneyInput value={f.credit_limit || ""} onValueChange={v => updatePresetField("credit_limit", v)} />
              </div>
              <div className="space-y-2">
                <Label className="text-sm font-medium">Due Date</Label>
                <DatePickerInput value={f.due_date ? new Date(f.due_date) : undefined} onChange={v => updatePresetField("due_date", v?.toISOString())} />
              </div>
              <div className="space-y-2">
                <Label className="text-sm font-medium">Card Last 4 digits (Optional)</Label>
                <Input maxLength={4} value={f.last_4 || ""} onChange={e => updatePresetField("last_4", e.target.value)} />
              </div>
            </>
          )}

          {instrumentPreset === "cash_in_hand" && (
            <>
              <div className="space-y-2">
                <Label className="text-sm font-medium">Location/Description (Optional)</Label>
                <Input value={f.location || ""} onChange={e => updatePresetField("location", e.target.value)} />
              </div>
            </>
          )}
        </div>
      )}
    </div>
  );
}
