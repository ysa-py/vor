// Package engine is the Go interpreter of the shared Vor decision model —
// the same algorithm as the Rust reference core/vor-core (bandit.rs,
// selector.rs, threat.rs, isp.rs) and the Kotlin port
// (android/app/.../data/ai/VorCoreKt.kt).
//
// All ports are pinned to the SAME shared conformance vectors
// (core/vor-core/tests/vectors/decision-vectors.json) so the desktop's
// "Auto (AI)" engine selector provably agrees with the shared library —
// one decision model, vector-locked ports, no drifting forks.
package engine

import (
	"encoding/json"
	"math"
	"sort"
	"strings"
)

// Constants mirrored from the Rust reference.
const (
	defaultAlpha             = 1.414
	defaultDecay             = 0.95
	recentWindow             = 100
	championThresholdDefault = 0.6
)

// ArmStats is per-arm bandit statistics.
type ArmStats struct {
	Pulls         uint64    `json:"pulls"`
	RewardSum     float64   `json:"reward_sum"`
	RecentRewards []float64 `json:"recent_rewards"`
}

// BanditState is the UCB1 bandit state (JSON field names match Rust).
type BanditState struct {
	Arms       map[string]*ArmStats `json:"arms"`
	TotalPulls uint64               `json:"total_pulls"`
	Alpha      float64              `json:"alpha"`
	Decay      float64              `json:"decay"`
}

// NewBandit creates an empty bandit with reference defaults.
func NewBandit() *BanditState {
	return &BanditState{
		Arms:  map[string]*ArmStats{},
		Alpha: defaultAlpha, Decay: defaultDecay,
	}
}

// ArmScore is the UCB1 score (0 for unpulled arms).
func (b *BanditState) ArmScore(id string) float64 {
	stats, ok := b.Arms[id]
	if !ok || stats.Pulls == 0 {
		return 0
	}
	average := stats.RewardSum / float64(stats.Pulls)
	total := math.Max(float64(b.TotalPulls), 1)
	exploration := b.Alpha * math.Sqrt(2*math.Log(total)/float64(stats.Pulls))
	return average + exploration
}

// Select is UCB1 selection; unpulled arms first, ties lexicographic.
func (b *BanditState) Select() (string, bool) {
	best := ""
	bestScore := math.Inf(-1)
	found := false
	ids := make([]string, 0, len(b.Arms))
	for id := range b.Arms {
		ids = append(ids, id)
	}
	sort.Strings(ids)
	for _, id := range ids {
		stats := b.Arms[id]
		score := math.Inf(1)
		if stats.Pulls > 0 {
			score = b.ArmScore(id)
		}
		if score > bestScore {
			bestScore = score
			best = id
			found = true
		}
	}
	return best, found
}

// Update folds a reward into an arm.
func (b *BanditState) Update(armID string, reward float64) {
	stats, ok := b.Arms[armID]
	if !ok {
		stats = &ArmStats{}
		b.Arms[armID] = stats
	}
	stats.RewardSum = stats.RewardSum*b.Decay + reward
	stats.Pulls++
	stats.RecentRewards = append(stats.RecentRewards, reward)
	if len(stats.RecentRewards) > recentWindow {
		stats.RecentRewards = stats.RecentRewards[1:]
	}
	b.TotalPulls++
}

// ThreatObservation mirrors threat.rs.
type ThreatObservation struct {
	RstAfterHello      uint64 `json:"rst_after_hello"`
	CleanConnections   uint64 `json:"clean_connections"`
	DnsPoisonAnswers   uint64 `json:"dns_poison_answers"`
	HttpBlockResponses uint64 `json:"http_block_responses"`
	SniFilterEvents    uint64 `json:"sni_filter_events"`
}

// Level computes the threat level wire name.
func (o ThreatObservation) Level() string {
	blocking := o.RstAfterHello + o.SniFilterEvents
	total := blocking + maxU64(o.CleanConnections, 1)
	if o.DnsPoisonAnswers >= 3 || o.HttpBlockResponses >= 5 {
		return "blackout"
	}
	if blocking == 0 && o.DnsPoisonAnswers == 0 && o.HttpBlockResponses == 0 {
		return "none"
	}
	ratio := float64(blocking) / float64(total)
	switch {
	case ratio >= 0.8:
		return "blackout"
	case o.SniFilterEvents > 0 || ratio >= 0.5:
		return "active_v2"
	case blocking > 0:
		return "active_v1"
	default:
		return "passive"
	}
}

func maxU64(a, b uint64) uint64 {
	if a > b {
		return a
	}
	return b
}

// LearnedEntry is one Q-table entry per network fingerprint.
type LearnedEntry struct {
	Champion       string  `json:"champion"`
	ChampionReward float64 `json:"champion_reward"`
	LastGood       string  `json:"last_good"`
	UpdatedAt      int64   `json:"updated_at"`
}

// SelectorState mirrors selector.rs SelectorState.
type SelectorState struct {
	Bandit            *BanditState             `json:"bandit"`
	Q                 map[string]*LearnedEntry `json:"q"`
	ChampionThreshold float64                  `json:"champion_threshold"`
}

// NewSelectorState creates the default state.
func NewSelectorState() *SelectorState {
	return &SelectorState{
		Bandit:            NewBandit(),
		Q:                 map[string]*LearnedEntry{},
		ChampionThreshold: championThresholdDefault,
	}
}

// ParseSelectorState decodes state JSON (nil on error).
func ParseSelectorState(text string) *SelectorState {
	var state SelectorState
	if err := json.Unmarshal([]byte(text), &state); err != nil {
		return nil
	}
	if state.Bandit == nil {
		state.Bandit = NewBandit()
	}
	if state.Bandit.Arms == nil {
		state.Bandit.Arms = map[string]*ArmStats{}
	}
	if state.Q == nil {
		state.Q = map[string]*LearnedEntry{}
	}
	if state.ChampionThreshold == 0 {
		state.ChampionThreshold = championThresholdDefault
	}
	return &state
}

// DecisionContext mirrors selector.rs DecisionContext.
type DecisionContext struct {
	Carrier          string            `json:"carrier"`
	Fingerprint      string            `json:"fingerprint"`
	Threat           ThreatObservation `json:"threat"`
	AvailableEngines []string          `json:"available_engines"`
	PinnedEngines    []string          `json:"pinned_engines"`
	ThreatOverride   *string           `json:"threat_override"`
}

// ParseDecisionContext decodes context JSON (nil on error).
func ParseDecisionContext(text string) *DecisionContext {
	var context DecisionContext
	if err := json.Unmarshal([]byte(text), &context); err != nil {
		return nil
	}
	return &context
}

// Outcome is one probe result.
type Outcome struct {
	Fingerprint   string `json:"fingerprint"`
	Engine        string `json:"engine"`
	Strategy      string `json:"strategy"`
	Success       bool   `json:"success"`
	RttMs         uint64 `json:"rtt_ms"`
	DpiKill       bool   `json:"dpi_kill"`
	ThroughputBps uint64 `json:"throughput_bps"`
}

// ParseOutcome decodes outcome JSON (nil on error).
func ParseOutcome(text string) *Outcome {
	var outcome Outcome
	if err := json.Unmarshal([]byte(text), &outcome); err != nil {
		return nil
	}
	return &outcome
}

// RewardFor is the shared reward formula (mirrors selector.rs).
func RewardFor(outcome *Outcome) float64 {
	if !outcome.Success {
		return 0
	}
	latency := 1.0 / (1.0 + float64(outcome.RttMs)/500.0)
	throughput := float64(outcome.ThroughputBps) / (float64(outcome.ThroughputBps) + 1_000_000)
	reward := 0.5 + 0.3*latency + 0.2*throughput
	if outcome.DpiKill {
		reward *= 0.5
	}
	return reward
}

// Decision is the output (JSON field names match Rust).
type Decision struct {
	Engine          string         `json:"engine"`
	Strategies      []string       `json:"strategies"`
	FragmentDelayMs int            `json:"fragment_delay_ms"`
	FakeSni         string         `json:"fake_sni"`
	Edges           []DecisionEdge `json:"edges"`
	TrafficShaping  bool           `json:"traffic_shaping"`
	ShaperLevel     string         `json:"shaper_level"`
	ThreatLevel     string         `json:"threat_level"`
	IsChampion      bool           `json:"is_champion"`
	Ranking         []RankedArm    `json:"ranking"`
}

// DecisionEdge is one recommended edge.
type DecisionEdge struct {
	IP       string `json:"ip"`
	Port     int    `json:"port"`
	MaxSplit int    `json:"max_split"`
	Label    string `json:"label"`
}

// RankedArm is one bandit ranking row.
type RankedArm struct {
	Arm   string  `json:"arm"`
	Score float64 `json:"score"`
}

// CarrierPreset mirrors isp.rs (the subset the desktop uses).
type CarrierPreset struct {
	Key                string         `json:"key"`
	MatchAsns          []string       `json:"match_asns"`
	DefaultStrategy    string         `json:"default_strategy"`
	FakeSni            string         `json:"fake_sni"`
	FragmentStrategies []string       `json:"fragment_strategies"`
	Edges              []DecisionEdge `json:"edges"`
	NeedsShaping       bool           `json:"needs_traffic_shaping"`
}

var carrierPresets []CarrierPreset

// SetCarrierPresets injects the shared JSON (single source:
// core/vor-core/data/carrier-presets.json — callers read it from disk once).
func SetCarrierPresets(sharedJson []byte) error {
	var root struct {
		Carriers []CarrierPreset `json:"carriers"`
	}
	if err := json.Unmarshal(sharedJson, &root); err != nil {
		return err
	}
	carrierPresets = root.Carriers
	return nil
}

// CarrierByKeyOrAsn resolves a carrier preset.
func CarrierByKeyOrAsn(keyOrAsn string) *CarrierPreset {
	normalized := strings.ToLower(strings.TrimSpace(keyOrAsn))
	asn := strings.ToUpper(strings.TrimSpace(keyOrAsn))
	asn = strings.TrimPrefix(asn, "AS")
	for i := range carrierPresets {
		carrier := &carrierPresets[i]
		if carrier.Key == normalized {
			return carrier
		}
		for _, candidate := range carrier.MatchAsns {
			if strings.TrimPrefix(strings.ToUpper(candidate), "AS") == asn {
				return carrier
			}
		}
	}
	return nil
}

// RecommendedStrategies is the best-first strategy ladder ending in raw.
func RecommendedStrategies(carrier *CarrierPreset) []string {
	strategies := make([]string, 0, len(carrier.FragmentStrategies)+2)
	for _, name := range carrier.FragmentStrategies {
		if name != "raw" {
			strategies = append(strategies, name)
		}
	}
	definition := normalizeStrategy(carrier.DefaultStrategy)
	if definition != "raw" && !contains(strategies, definition) {
		strategies = append([]string{definition}, strategies...)
	}
	if !contains(strategies, "random_split") {
		strategies = append(strategies, "random_split")
	}
	strategies = append(strategies, "raw")
	return strategies
}

func normalizeStrategy(name string) string {
	switch strings.ToLower(name) {
	case "sni_split":
		return "sni_split"
	case "record_split":
		return "record_split"
	case "random_split":
		return "random_split"
	default:
		return strings.ToLower(name)
	}
}

func contains(list []string, value string) bool {
	for _, item := range list {
		if item == value {
			return true
		}
	}
	return false
}

// KnownEngines matches the selector vocabulary (selector.rs KNOWN_ENGINES).
var KnownEngines = []string{
	"xray", "singbox", "sni-tunnel", "pattern", "dns-tunnel-dnstt",
	"dns-tunnel-masterdns", "tor", "psiphon", "mitm-fronting",
}

func fragmentApplicable(engine string) bool {
	switch engine {
	case "sni-tunnel", "pattern", "xray", "mitm-fronting":
		return true
	}
	return false
}

// Decide computes the next decision (mirrors selector.rs decide()).
func Decide(state *SelectorState, context *DecisionContext) *Decision {
	var carrier *CarrierPreset
	if context.Carrier != "" {
		carrier = CarrierByKeyOrAsn(context.Carrier)
	}

	threatLevel := context.Threat.Level()
	if context.ThreatOverride != nil {
		threatLevel = threatFromWire(*context.ThreatOverride)
	}

	strategies := []string{"sni_split", "record_split", "full10", "random_split", "raw"}
	if carrier != nil {
		strategies = RecommendedStrategies(carrier)
	}

	available := context.AvailableEngines
	if len(available) == 0 {
		available = KnownEngines
	}

	arms := make([]string, 0, len(available)*len(strategies))
	for _, engine := range available {
		if fragmentApplicable(engine) {
			for _, strategy := range strategies {
				arms = append(arms, engine+"|"+strategy)
			}
		} else {
			arms = append(arms, engine+"|raw")
		}
	}

	bandit := state.Bandit
	for _, arm := range arms {
		if _, ok := bandit.Arms[arm]; !ok {
			bandit.Arms[arm] = &ArmStats{}
		}
	}
	armSet := make(map[string]bool, len(arms))
	for _, arm := range arms {
		armSet[arm] = true
	}
	for id := range bandit.Arms {
		if !armSet[id] {
			delete(bandit.Arms, id)
		}
	}

	fingerprint := context.Fingerprint
	if fingerprint == "" {
		fingerprint = "default"
	}
	chosenArm := ""
	isChampion := false
	if learned, ok := state.Q[fingerprint]; ok &&
		learned.ChampionReward >= state.ChampionThreshold && armSet[learned.Champion] {
		chosenArm = learned.Champion
		isChampion = true
	}
	if chosenArm == "" {
		if bandit.TotalPulls == 0 {
			// Fresh: highest-priority strategy first, lexicographic engine.
			bestRank := math.MaxInt32
			for _, arm := range arms {
				strategy := arm[strings.IndexByte(arm, '|')+1:]
				rank := indexOf(strategies, strategy)
				if rank < 0 {
					rank = math.MaxInt32
				}
				if rank < bestRank || (rank == bestRank && arm < chosenArm) {
					bestRank = rank
					chosenArm = arm
				}
			}
		} else if selection, ok := bandit.Select(); ok {
			chosenArm = selection
		}
	}
	if chosenArm == "" {
		chosenArm = "xray|raw"
	}
	engineID := chosenArm[:strings.IndexByte(chosenArm, '|')]
	strategy := chosenArm[strings.IndexByte(chosenArm, '|')+1:]

	fakeSni := "www.speedtest.net"
	var edges []DecisionEdge
	if carrier != nil {
		fakeSni = carrier.FakeSni
		edges = carrier.Edges
	}

	fragmentDelay := 5
	if threatRequiresAggressive(threatLevel) {
		fragmentDelay = 0
	}

	ispNeedsShaping := false
	if carrier != nil {
		ispNeedsShaping = carrier.NeedsShaping
	}
	shaper := "passthrough"
	switch {
	case !threatLayer2(threatLevel, ispNeedsShaping):
		shaper = "passthrough"
	case threatRequiresAggressive(threatLevel):
		shaper = "maximum"
	case threatLevel != "none":
		shaper = "full"
	default:
		shaper = "light"
	}

	ordered := make([]string, 0, len(strategies))
	if strategy != "raw" {
		ordered = append(ordered, strategy)
	}
	for _, item := range strategies {
		if item != strategy {
			ordered = append(ordered, item)
		}
	}

	ranking := make([]RankedArm, 0, len(bandit.Arms))
	for id := range bandit.Arms {
		ranking = append(ranking, RankedArm{Arm: id, Score: round3(bandit.ArmScore(id))})
	}
	sort.Slice(ranking, func(i, j int) bool {
		if ranking[i].Score != ranking[j].Score {
			return ranking[i].Score > ranking[j].Score
		}
		return ranking[i].Arm < ranking[j].Arm
	})

	return &Decision{
		Engine:          engineID,
		Strategies:      ordered,
		FragmentDelayMs: fragmentDelay,
		FakeSni:         fakeSni,
		Edges:           edges,
		TrafficShaping:  shaper != "passthrough",
		ShaperLevel:     shaper,
		ThreatLevel:     threatLevel,
		IsChampion:      isChampion,
		Ranking:         ranking,
	}
}

// Observe folds an outcome into the state (mirrors selector.rs observe()).
func Observe(state *SelectorState, outcome *Outcome, nowEpoch int64) *SelectorState {
	fingerprint := outcome.Fingerprint
	if fingerprint == "" {
		fingerprint = "default"
	}
	strategy := outcome.Strategy
	if strategy == "" {
		strategy = "raw"
	}
	arm := outcome.Engine + "|" + strategy
	reward := RewardFor(outcome)

	if _, ok := state.Bandit.Arms[arm]; !ok {
		state.Bandit.Arms[arm] = &ArmStats{}
	}
	state.Bandit.Update(arm, reward)

	entry, ok := state.Q[fingerprint]
	if !ok {
		entry = &LearnedEntry{Champion: arm, LastGood: "xray|raw"}
		state.Q[fingerprint] = entry
	}
	average := 0.0
	if stats := state.Bandit.Arms[arm]; stats != nil && stats.Pulls > 0 {
		average = stats.RewardSum / float64(stats.Pulls)
	}
	if entry.Champion != arm && average > entry.ChampionReward {
		entry.Champion = arm
		entry.ChampionReward = round3(average)
	} else if entry.Champion == arm {
		entry.ChampionReward = round3(average)
	}
	if outcome.Success {
		entry.LastGood = arm
	}
	entry.UpdatedAt = nowEpoch
	return state
}

func threatFromWire(value string) string {
	switch value {
	case "passive":
		return "passive"
	case "active_v1", "active-v1", "ActiveV1":
		return "active_v1"
	case "active_v2", "active-v2", "ActiveV2":
		return "active_v2"
	case "blackout", "complete_blackout", "CompleteBlackout":
		return "blackout"
	default:
		return "none"
	}
}

func threatRequiresAggressive(level string) bool {
	return level == "active_v2" || level == "blackout"
}

func threatLayer2(level string, ispNeedsShaping bool) bool {
	switch level {
	case "none":
		return false
	case "passive":
		return ispNeedsShaping
	default:
		return true
	}
}

func indexOf(list []string, value string) int {
	for i, item := range list {
		if item == value {
			return i
		}
	}
	return -1
}

func round3(value float64) float64 {
	return math.Round(value*1000) / 1000
}
