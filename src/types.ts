export interface ImportantFile {
  path: string;
  why_it_matters: string;
  confidence_notes: string;
  possible_gaps_or_uncertainties: string;
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
