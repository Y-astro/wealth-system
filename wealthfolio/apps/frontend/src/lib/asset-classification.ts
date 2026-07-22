export type LiquidityClass = 'LIQUID' | 'ILLIQUID';
export type RiskClass = 'SAFE' | 'MODERATE' | 'AGGRESSIVE';

export function classifyLiquidity(kind: string, metadata?: Record<string, unknown>): LiquidityClass | null {
  const k = kind.toLowerCase();
  
  if (['property', 'vehicle', 'collectible', 'private_equity'].includes(k)) {
    return 'ILLIQUID';
  }
  
  if (['precious', 'investment', 'cash'].includes(k)) {
    return 'LIQUID';
  }
  
  if (k === 'liability') {
    return null;
  }
  
  if (k === 'other') {
    const preset = (metadata?.instrument_preset as string)?.toLowerCase();
    if (preset && ['ppf', 'epf', 'nps'].includes(preset)) {
      return 'ILLIQUID';
    }
    if (preset && ['fd', 'rd', 'sgb', 'cash'].includes(preset)) {
      return 'LIQUID';
    }
    return 'LIQUID';
  }
  
  return 'LIQUID';
}

export function classifyRisk(kind: string, metadata?: Record<string, unknown>): RiskClass | null {
  const k = kind.toLowerCase();
  
  if (['property', 'precious'].includes(k)) {
    return 'MODERATE';
  }
  
  if (['vehicle', 'collectible', 'investment'].includes(k)) {
    return 'AGGRESSIVE';
  }
  
  if (k === 'liability') {
    return null;
  }
  
  if (k === 'other') {
    const preset = (metadata?.instrument_preset as string)?.toLowerCase();
    if (preset && ['ppf', 'epf', 'fd', 'rd'].includes(preset)) {
      return 'SAFE';
    }
    return 'MODERATE'; 
  }
  
  return 'MODERATE';
}

export function isStaleValuation(valuationDate: string, thresholdDays: number = 90): boolean {
  if (!valuationDate) return false;
  
  const date = new Date(valuationDate);
  if (isNaN(date.getTime())) return false;
  
  const now = new Date();
  const diffTime = now.getTime() - date.getTime();
  const diffDays = diffTime / (1000 * 60 * 60 * 24); 
  
  return diffDays > thresholdDays;
}
