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

export interface AiOpportunitiesResult {
  payload: OpportunityPayload;
  from_cache: boolean;
}

/** Proof snippet: inferred CLI, file sample, or UI description — makes the case study feel real. */
export interface CaseStudyProofBlock {
  kind: string;
  title: string;
  body: string;
}

/** Client-winning case study (from stored analysis + optional writer profile). */
export interface CaseStudyPayload {
  title: string;
  problem: string;
  why_it_mattered: string;
  approach: string;
  solution: string;
  outcome: string;
  outcome_basis: string;
  narrative: string;
  quote_ready_one_liner: string;
  what_we_built: string[];
  proof_blocks: CaseStudyProofBlock[];
}

export interface AiCaseStudyResult {
  payload: CaseStudyPayload;
  from_cache: boolean;
}

/** Local onboarding — injected into case study generation when set. */
export interface UserProfile {
  role?: string | null;
  what_i_build: string[];
  app_goal?: string | null;
}

export function isUserProfileFilled(p: UserProfile | null | undefined): boolean {
  if (!p) return false;
  if (p.role && String(p.role).length > 0) return true;
  if (p.what_i_build?.length) return true;
  if (p.app_goal && String(p.app_goal).length > 0) return true;
  return false;
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
  /** Non-technical: user experience and workflow. */
  what_it_actually_does?: string;
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
  positioning_label?: string;
  interview_talking_points?: string;
  portfolio_positioning?: string;
  /** Three strings: problem / solution / insight post angles. */
  social_content_angles?: string[];
  suggested_social_post?: string;
}

/** From `list_projects` only — id, name, summary; no path/stack/analysis. */
export interface ProjectListItem {
  id: number;
  name: string;
  one_line_summary: string;
  last_analyzed_at: string | null;
  is_pinned: boolean;
}

/** Full project row (e.g. `get_project` / detail). */
export interface ProjectRow {
  id: number;
  name: string;
  path: string;
  detected_stack: string[];
  one_line_summary: string;
  last_analyzed_at: string | null;
  created_at: string;
  is_pinned: boolean;
}

export interface ProjectEvolutionEntry {
  id: number;
  label: string;
  new_features: string[];
  summary: string;
  created_at: string;
}

export interface ProjectDetail extends ProjectRow {
  analysis: AnalysisPayload | null;
  file_index_sample: string[];
  raw_file_list_truncated: boolean;
  evolutions?: ProjectEvolutionEntry[];
}

/** AI-ranked projects for your current goal */
export interface RankedPick {
  project_id: number;
  project_name: string;
  rationale: string;
}

export interface TopProjectsPayload {
  picks: RankedPick[];
}

export interface IncrementalUpdatePayload {
  version_label: string;
  what_changed_overview: string;
  new_features: string[];
  improvements: string[];
}

export interface IncrementalUpdateResult {
  evolution_id: number;
  payload: IncrementalUpdatePayload;
}

export interface EvolutionSuggestion {
  title: string;
  why: string;
  build_notes: string;
}

export interface EvolutionSuggestionsPayload {
  suggestions: EvolutionSuggestion[];
}

export interface PositioningPayload {
  category: string;
  primary_audience: string;
  one_sentence_anchor: string;
}

export interface RuntimeStatus {
  hasApiKey: boolean;
  hasProfile: boolean;
}

/** AI provider + models (keys are never returned; flags show if a key is configured). */
export interface AiSettingsPublic {
  provider: string;
  anthropicModel: string;
  openaiModel: string;
  hasAnthropicKey: boolean;
  hasOpenaiKey: boolean;
}

export interface ProjectImportancePayload {
  top_insights: string[];
}

export interface ExportBundleResult {
  writtenFiles: string[];
}

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
