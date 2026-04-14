export interface ImportantFile {
  path: string;
  why_it_matters: string;
  confidence_notes: string;
  possible_gaps_or_uncertainties: string;
}

export interface Opportunity {
  title: string;
  what_it_is: string;
  problem: string;
  why_this_problem_is_real_now: string;
  target_customer: string;
  who_exactly_to_contact: string;
  how_to_package: string;
  pricing_logic: string;
  distribution_strategy: string[];
  first_3_steps_to_validate: string[];
  risk_level: string;
  why_this_could_fail: string;
}

export interface OpportunityPayload {
  opportunities: Opportunity[];
}

/** Client-winning case study (from stored analysis). */
export interface CaseStudyPayload {
  title: string;
  problem: string;
  solution: string;
  outcome: string;
  outcome_basis: string;
  narrative: string;
  linkedin_hook: string;
  quote_ready_one_liner: string;
  what_we_built: string[];
}

export interface ProductIntelligence {
  category: string;
  target_users: string[];
  use_cases: string[];
  monetization_models: string[];
  distribution_channels: string[];
  product_stage: string;
  what_is_missing: string[];
  strengths: string[];
  risks: string[];
go_to_market?: {
  target_user: string;
  sell_as: string;
  where_to_sell: string[];
  first_steps: string[];
};
};

export interface AnalysisPayload {
  project_name: string;
  project_intent: string;
  when_built: string;
  one_line_summary: string;
  deep_explanation: string;
  /** Long-form narrative inside JSON; primary deep read. Omitted on analyses stored before this field existed. */
  full_narrative_explanation?: string;
  problem_it_solves: string;
  why_it_matters: string;
  core_features: string[];
  key_flows: string[];
  tech_stack: string[];
  architecture_overview: string;
  how_it_works_step_by_step: string[];
  design_decisions: string[];
  tradeoffs_and_limitations: string[];
  how_to_run: string;
  example_outputs: string[];
  important_files: ImportantFile[];
  /** Present after analyses that include the Product Intelligence layer; re-analyze older projects to populate. */
  product_intelligence?: ProductIntelligence;
}

export interface ProjectListItem {
  id: number;
  name: string;
  path: string;
  detected_stack: string[];
  one_line_summary: string;
  last_analyzed_at: string | null;
  created_at: string;
}

export interface ProjectDetail extends ProjectListItem {
  analysis: AnalysisPayload | null;
  file_index_sample: string[];
  raw_file_list_truncated: boolean;
}

/** Alias: matches Rust `ProjectRow` from `list_projects`. */
export type ProjectRow = ProjectListItem;

/** Persisted “Idea Project” (saved opportunity). */
export interface IdeaProject {
  id: number;
  source_project_id: number;
  source_project_name: string;
  title: string;
  what_it_is: string;
  problem: string;
  why_this_problem_is_real_now: string;
  target_customer: string;
  who_exactly_to_contact: string;
  how_to_package: string;
  pricing_logic: string;
  distribution_strategy: string[];
  first_3_steps_to_validate: string[];
  risk_level: string;
  why_this_could_fail: string;
  saved_at: string;
}
