export interface Issue {
  id: string;
  severity: "error" | "warning" | "info";
  message: string;
  path?: string;
}

export interface ValidationResult {
  valid: boolean;
  issues: Issue[];
}

/**
 * Validate an OpenRTB JSON payload (bid request or response).
 */
export function validate(input: string): Promise<ValidationResult>;
