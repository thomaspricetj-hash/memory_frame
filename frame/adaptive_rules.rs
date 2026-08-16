use crate::frame::MemoryFrame;
use crate::layers::LayerId;
use crate::config::MemoryPolicy;
use crate::frame::SliceData;

/// High-level categories of adaptive rules.
#[derive(Debug, Clone, Copy)]
pub enum AdaptiveRule {
    Verification,
    Stability,
    Consistency,
    Temporal,
    Semantic,
    Delta,
    Confidence,
}

/// Scoring breakdown for a single evaluation pass.
#[derive(Debug, Clone)]
pub struct AdaptiveScore {
    pub total: f32,
    pub correct_answer: f32,
    pub confidence: f32,
    pub context_linked: f32,
    pub adaptive_memory: f32,
    pub semantic_alignment: f32,
    pub delta_match: f32,
}

/// Central adaptive rule engine.
/// This is the "governor" that sits above MemoryFrame + CrossConnect.
#[derive(Debug, Default)]
pub struct AdaptiveRuleEngine {
    pub last_score: Option<AdaptiveScore>,
}

impl AdaptiveRuleEngine {
    /// Evaluate the entire frame and update internal score.
    pub fn evaluate_frame(
        &mut self,
        frame: &MemoryFrame,
        policy: &MemoryPolicy,
    ) -> AdaptiveScore {
        let mut score = AdaptiveScore {
            total: 0.0,
            correct_answer: 0.0,
            confidence: 0.0,
            context_linked: 0.0,
            adaptive_memory: 0.0,
            semantic_alignment: 0.0,
            delta_match: 0.0,
        };

        // Apply rule groups
        self.apply_verification_rules(frame, &mut score);
        self.apply_stability_rules(frame, &mut score);
        self.apply_consistency_rules(frame, &mut score);
        self.apply_temporal_rules(frame, policy, &mut score);
        self.apply_semantic_rules(frame, &mut score);
        self.apply_delta_rules(frame, &mut score);
        self.apply_confidence_rules(frame, &mut score);

        // Aggregate total
        score.total = score.correct_answer
            + score.confidence
            + score.context_linked
            + score.adaptive_memory
            + score.semantic_alignment
            + score.delta_match;

        self.last_score = Some(score.clone());
        score
    }

    // -------------------------------------------------------------------------
    // VERIFICATION RULES
    // -------------------------------------------------------------------------
    fn apply_verification_rules(&self, frame: &MemoryFrame, score: &mut AdaptiveScore) {
        let mut verified: f32 = 0.0;
        let mut total_semantic: f32 = 0.0;

        for (id, slice) in &frame.slices {
            if *id == LayerId::Semantic || *id == LayerId::Declarative {
                total_semantic += 1.0;
                if matches!(slice.data, SliceData::Semantic(_) | SliceData::Declarative(_)) {
                    verified += 1.0;
                }
            }
        }

        if total_semantic > 0.0 {
            let ratio: f32 = verified / total_semantic;
            score.correct_answer += ratio.clamp(0.0, 1.0);
        }
    }

    // -------------------------------------------------------------------------
    // STABILITY RULES
    // -------------------------------------------------------------------------
    fn apply_stability_rules(&self, frame: &MemoryFrame, score: &mut AdaptiveScore) {
        let slice_count = frame.slices.len() as f32;
        score.adaptive_memory += (slice_count / 6.0).clamp(0.0, 1.0);
    }

    // -------------------------------------------------------------------------
    // CONSISTENCY RULES
    // -------------------------------------------------------------------------
    fn apply_consistency_rules(&self, frame: &MemoryFrame, score: &mut AdaptiveScore) {
        let sem = frame.diagonal_semantic_surface;
        let conf = frame.diagonal_confidence_surface;

        score.semantic_alignment += sem.clamp(0.0, 1.5) / 1.5;
        score.context_linked += ((sem + conf) / 2.0).clamp(0.0, 1.5) / 1.5;
    }

    // -------------------------------------------------------------------------
    // TEMPORAL RULES
    // -------------------------------------------------------------------------
    fn apply_temporal_rules(
        &self,
        frame: &MemoryFrame,
        policy: &MemoryPolicy,
        score: &mut AdaptiveScore,
    ) {
        let align = frame.temporal_alignment_score;
        let decay = frame.temporal_decay_factor;
        let floor = policy.decay.confidence_floor;

        let temporal_conf = ((align * decay) * (1.0 - floor)).clamp(0.0, 1.5) / 1.5;
        score.confidence += temporal_conf;
    }

    // -------------------------------------------------------------------------
    // SEMANTIC RULES
    // -------------------------------------------------------------------------
    fn apply_semantic_rules(&self, frame: &MemoryFrame, score: &mut AdaptiveScore) {
        let mut sem_sum: f32 = 0.0;
        let mut count: f32 = 0.0;

        for slice in frame.slices.values() {
            sem_sum += slice.diagonal_semantic_surface();
            count += 1.0;
        }

        if count > 0.0 {
            score.semantic_alignment += (sem_sum / count).clamp(0.0, 1.5) / 1.5;
        }
    }

    // -------------------------------------------------------------------------
    // DELTA RULES
    // -------------------------------------------------------------------------
    fn apply_delta_rules(&self, frame: &MemoryFrame, score: &mut AdaptiveScore) {
        let collapse = frame.diagonal_collapse_influence;
        score.delta_match += (collapse / 2.0).clamp(0.0, 1.0);
    }

    // -------------------------------------------------------------------------
    // CONFIDENCE RULES
    // -------------------------------------------------------------------------
    fn apply_confidence_rules(&self, frame: &MemoryFrame, score: &mut AdaptiveScore) {
        score.confidence += frame.global_confidence().clamp(0.0, 1.0);
    }

    // -------------------------------------------------------------------------
    // PUBLIC HELPERS
    // -------------------------------------------------------------------------
    pub fn score_frame_total(
        &mut self,
        frame: &MemoryFrame,
        policy: &MemoryPolicy,
    ) -> f32 {
        self.evaluate_frame(frame, policy).total
    }

    pub fn apply_to_frame(
        &mut self,
        frame: &mut MemoryFrame,
        policy: &MemoryPolicy,
    ) -> AdaptiveScore {
        self.evaluate_frame(frame, policy)
    }
}







