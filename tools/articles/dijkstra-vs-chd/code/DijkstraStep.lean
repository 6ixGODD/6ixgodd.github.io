import Mathlib.Data.ENNReal.Basic

/-
This file formalizes TWO LOCAL LEMMAS, not a complete Dijkstra implementation.
It has not been compiled in the artifact-generation environment (Lean unavailable).
Use a compatible Lean/Mathlib project to check it before publishing checked claims.
-/
namespace ShortestPathNotes

-- The graph/path development must supply `crossing`; it is not proved here.
theorem settle_from_boundary
    {V : Type*}
    (settled : Set V) (d delta : V → ℝ≥0∞) (u : V)
    (upper : ∀ v, delta v ≤ d v)
    (minimum : ∀ v, v ∉ settled → d u ≤ d v)
    (crossing : ∃ y, y ∉ settled ∧ d y ≤ delta y ∧ delta y ≤ delta u) :
    d u = delta u := by
  rcases crossing with ⟨y, hy, hy_exact, hy_prefix⟩
  apply le_antisymm
  · exact le_trans (minimum y hy) (le_trans hy_exact hy_prefix)
  · exact upper u

-- Extending an actual path cannot create an estimate below the true distance.
theorem relax_preserves_upper
    (delta_u delta_v du dv w : ℝ≥0∞)
    (hu : delta_u ≤ du) (hv : delta_v ≤ dv)
    (edge_bound : delta_v ≤ delta_u + w) :
    delta_v ≤ min dv (du + w) := by
  exact le_min hv (le_trans edge_bound (add_le_add_right hu w))

#print axioms settle_from_boundary
#print axioms relax_preserves_upper
end ShortestPathNotes
