package engine

import (
	"encoding/binary"
	"encoding/json"
	"os"
	"testing"
)

// Shared cross-platform decision vectors
// (core/vor-core/tests/vectors/decision-vectors.json) — the same file runs
// against the Rust reference and the Kotlin port, pinning all three.

type vectorsFile struct {
	Metadata struct {
		Now             string  `json:"now"`
		RewardTolerance float64 `json:"reward_tolerance"`
	} `json:"metadata"`
	RewardCases []struct {
		Name     string  `json:"name"`
		Outcome  Outcome `json:"outcome"`
		Expected float64 `json:"expected_reward"`
	} `json:"reward_cases"`
	ThreatCases []struct {
		Name        string            `json:"name"`
		Observation ThreatObservation `json:"observation"`
		Expected    string            `json:"expected_level"`
	} `json:"threat_cases"`
	DecisionCases []struct {
		Name     string          `json:"name"`
		State    json.RawMessage `json:"state"`
		Context  DecisionContext `json:"context"`
		Expected struct {
			Engine          string   `json:"engine"`
			FirstStrategy   string   `json:"first_strategy"`
			Strategies      []string `json:"strategies"`
			FragmentDelayMs int      `json:"fragment_delay_ms"`
			FakeSni         string   `json:"fake_sni"`
			EdgeCount       int      `json:"edge_count"`
			FirstEdge       *struct {
				IP       string `json:"ip"`
				Port     int    `json:"port"`
				MaxSplit int    `json:"max_split"`
			} `json:"first_edge"`
			TrafficShaping bool   `json:"traffic_shaping"`
			ShaperLevel    string `json:"shaper_level"`
			ThreatLevel    string `json:"threat_level"`
			IsChampion     bool   `json:"is_champion"`
		} `json:"expected"`
	} `json:"decision_cases"`
}

func loadVectors(t *testing.T) *vectorsFile {
	t.Helper()
	raw, err := os.ReadFile("testdata/decision-vectors.json")
	if err != nil {
		t.Fatalf("vectors: %v", err)
	}
	var vectors vectorsFile
	if err := json.Unmarshal(raw, &vectors); err != nil {
		t.Fatalf("vectors json: %v", err)
	}
	return &vectors
}

func TestMain(m *testing.M) {
	// Load the shared carrier presets (same file as the Rust/Kotlin ports).
	if raw, err := os.ReadFile("testdata/carrier-presets.json"); err == nil {
		if err := SetCarrierPresets(raw); err != nil {
			panic(err)
		}
	} else if raw, err := os.ReadFile("../../../core/vor-core/data/carrier-presets.json"); err == nil {
		if err := SetCarrierPresets(raw); err != nil {
			panic(err)
		}
	}
	os.Exit(m.Run())
}

func TestRewardVectors(t *testing.T) {
	vectors := loadVectors(t)
	for _, testCase := range vectors.RewardCases {
		outcome := testCase.Outcome
		actual := RewardFor(&outcome)
		if diff := actual - testCase.Expected; diff < -vectors.Metadata.RewardTolerance || diff > vectors.Metadata.RewardTolerance {
			t.Errorf("case %s: got %f, want %f", testCase.Name, actual, testCase.Expected)
		}
	}
}

func TestThreatVectors(t *testing.T) {
	vectors := loadVectors(t)
	for _, testCase := range vectors.ThreatCases {
		if got := testCase.Observation.Level(); got != testCase.Expected {
			t.Errorf("case %s: got %s, want %s", testCase.Name, got, testCase.Expected)
		}
	}
}

func TestDecisionVectors(t *testing.T) {
	vectors := loadVectors(t)
	for _, testCase := range vectors.DecisionCases {
		var state *SelectorState
		if len(testCase.State) > 0 && string(testCase.State) != "null" {
			state = ParseSelectorState(string(testCase.State))
			if state == nil {
				t.Fatalf("case %s: state failed to parse", testCase.Name)
			}
		} else {
			state = NewSelectorState()
		}
		decision := Decide(state, &testCase.Context)
		expected := testCase.Expected

		if decision.Engine != expected.Engine {
			t.Errorf("case %s: engine %s != %s", testCase.Name, decision.Engine, expected.Engine)
		}
		if expected.FirstStrategy != "" && decision.Strategies[0] != expected.FirstStrategy {
			t.Errorf("case %s: first strategy %s != %s", testCase.Name, decision.Strategies[0], expected.FirstStrategy)
		}
		if len(expected.Strategies) > 0 {
			if len(decision.Strategies) != len(expected.Strategies) {
				t.Errorf("case %s: strategies len %d != %d", testCase.Name, len(decision.Strategies), len(expected.Strategies))
			} else {
				for i, want := range expected.Strategies {
					if decision.Strategies[i] != want {
						t.Errorf("case %s: strategies[%d] %s != %s", testCase.Name, i, decision.Strategies[i], want)
					}
				}
			}
		}
		if expected.FragmentDelayMs != 0 && decision.FragmentDelayMs != expected.FragmentDelayMs {
			t.Errorf("case %s: delay %d != %d", testCase.Name, decision.FragmentDelayMs, expected.FragmentDelayMs)
		}
		if expected.FakeSni != "" && decision.FakeSni != expected.FakeSni {
			t.Errorf("case %s: fake sni %s != %s", testCase.Name, decision.FakeSni, expected.FakeSni)
		}
		if expected.EdgeCount > 0 && len(decision.Edges) != expected.EdgeCount {
			t.Errorf("case %s: edges %d != %d", testCase.Name, len(decision.Edges), expected.EdgeCount)
		}
		if expected.FirstEdge != nil {
			edge := decision.Edges[0]
			if edge.IP != expected.FirstEdge.IP || edge.Port != expected.FirstEdge.Port || edge.MaxSplit != expected.FirstEdge.MaxSplit {
				t.Errorf("case %s: first edge %+v != %+v", testCase.Name, edge, expected.FirstEdge)
			}
		}
		if expected.ShaperLevel != "" && decision.ShaperLevel != expected.ShaperLevel {
			t.Errorf("case %s: shaper %s != %s", testCase.Name, decision.ShaperLevel, expected.ShaperLevel)
		}
		if expected.ThreatLevel != "" && decision.ThreatLevel != expected.ThreatLevel {
			t.Errorf("case %s: threat %s != %s", testCase.Name, decision.ThreatLevel, expected.ThreatLevel)
		}
		if decision.IsChampion != expected.IsChampion {
			t.Errorf("case %s: champion %v != %v", testCase.Name, decision.IsChampion, expected.IsChampion)
		}
	}
}

func TestStateRoundTrip(t *testing.T) {
	state := NewSelectorState()
	Observe(state, &Outcome{
		Fingerprint: "fp", Engine: "xray", Strategy: "raw",
		Success: true, RttMs: 20,
	}, 1789041600)
	encoded, err := json.Marshal(state)
	if err != nil {
		t.Fatal(err)
	}
	back := ParseSelectorState(string(encoded))
	if back == nil {
		t.Fatal("round trip failed to parse")
	}
	if back.Q["fp"].Champion != "xray|raw" {
		t.Fatalf("champion %q", back.Q["fp"].Champion)
	}
}

func TestCarrierLookup(t *testing.T) {
	carrier := CarrierByKeyOrAsn("AS41689")
	if carrier == nil || carrier.Key != "mci" {
		t.Fatalf("AS41689 -> %+v", carrier)
	}
	if carrier.FakeSni != "www.speedtest.net" {
		t.Fatalf("mci fake sni %q", carrier.FakeSni)
	}
	irancell := CarrierByKeyOrAsn("irancell")
	if irancell == nil || irancell.FakeSni != "chatgpt.com" {
		t.Fatalf("irancell -> %+v", irancell)
	}
}

// ClientHello fixture shared with the pattern package test (kept here for
// the vector-driven invariants).
func TestBinaryFixtureSanity(t *testing.T) {
	_ = binary.BigEndian
}
