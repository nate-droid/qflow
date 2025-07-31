import React, { useState, useMemo, useEffect, useRef } from "react";

// --- Helper Hooks & Utils ---

/**
 * A custom hook to debounce a value.
 * This is useful to prevent rapid-firing events, like parsing code on every keystroke.
 * @param {any} value The value to debounce.
 * @param {number} delay The debounce delay in milliseconds.
 * @returns The debounced value.
 */
function useDebounce(value, delay) {
  const [debouncedValue, setDebouncedValue] = useState(value);
  useEffect(() => {
    const handler = setTimeout(() => {
      setDebouncedValue(value);
    }, delay);
    return () => {
      clearTimeout(handler);
    };
  }, [value, delay]);
  return debouncedValue;
}

/**
 * Parses QCL code to extract parameter definitions.
 * @param {string} code The QCL code string.
 * @returns {object} An object mapping parameter names to their values.
 */
function parseParameters(code) {
  const params = {};
  const regex = /\(defparam\s+'([a-zA-Z0-9_-]+)\s+([0-9.-]+)\)/g;
  let match;
  while ((match = regex.exec(code)) !== null) {
    params[match[1]] = parseFloat(match[2]);
  }
  return params;
}

/**
 * Parses QCL code to extract the first circuit definition.
 * @param {string} code The QCL code string.
 * @returns {object|null} A structured object representing the circuit or null if not found.
 */
function parseCircuit(code) {
  const circuitRegex =
    /\(defcircuit\s+'([a-zA-Z0-9_-]+)\s+\(\s*([\s\S]*?)\s*\)\)/;
  const circuitMatch = code.match(circuitRegex);

  if (!circuitMatch) return null;

  const name = circuitMatch[1];
  const body = circuitMatch[2];

  const gateRegex = /\(\s*([A-Z]+)\s+((?:'?[a-zA-Z0-9_.-]+\s*)+)\)/g;
  let gateMatch;
  const gates = [];
  let maxQubit = -1;

  while ((gateMatch = gateRegex.exec(body)) !== null) {
    const type = gateMatch[1];
    const args = gateMatch[2].trim().split(/\s+/);
    const gateQubits = args
      .map((arg) => parseInt(arg, 10))
      .filter((num) => !isNaN(num));

    if (gateQubits.length > 0) {
      maxQubit = Math.max(maxQubit, ...gateQubits);
    }

    gates.push({
      type,
      args,
      qubits: gateQubits,
    });
  }

  return {
    name,
    gates,
    numQubits: maxQubit + 1,
  };
}

//================================================================================
// --- QCL IDE COMPONENTS ---
//================================================================================

/**
 * A simple code editor with syntax highlighting for our custom QCL language.
 */
const QclCodeEditor = ({ code, setCode, errorLine }) => {
  const highlightedCode = useMemo(() => {
    const tokenDefs = [
      { type: "comment", regex: /;.*/, color: "#676e95" },
      {
        type: "command",
        regex: /\b(defparam|defcircuit|run|loop|let|write-file)\b/,
        color: "#c792ea",
      },
      { type: "symbol", regex: /'[a-zA-Z0-9_-]+/, color: "#f78c6c" },
      { type: "string", regex: /"[^"]*"/, color: "#c3e88d" },
      { type: "number", regex: /[0-9.-]+/, color: "#82aaff" },
      { type: "paren", regex: /[()]/, color: "#e2e8f0" },
    ];
    const combinedRegex = new RegExp(
      tokenDefs.map((t) => `(${t.regex.source})`).join("|"),
      "g",
    );

    return code.split("\n").map((line, lineIndex) => {
      const parts = line.split(combinedRegex).filter(Boolean);
      return (
        <div
          key={lineIndex}
          className={`line leading-6 ${lineIndex + 1 === errorLine ? "bg-red-900/30" : ""}`}
        >
          <span className="w-8 inline-block text-slate-600 text-right pr-4 select-none">
            {lineIndex + 1}
          </span>
          {parts.map((part, partIndex) => {
            for (const def of tokenDefs) {
              if (new RegExp(`^${def.regex.source}$`).test(part)) {
                return (
                  <span key={partIndex} style={{ color: def.color }}>
                    {part}
                  </span>
                );
              }
            }
            return <React.Fragment key={partIndex}>{part}</React.Fragment>;
          })}
        </div>
      );
    });
  }, [code, errorLine]);

  return (
    <div className="relative font-mono text-sm bg-slate-900 rounded-lg border border-slate-700 h-full flex flex-col">
      <div className="p-3 border-b border-slate-700 flex-shrink-0">
        <h3 className="text-base font-bold text-slate-200">QCL Code Editor</h3>
      </div>
      <div className="relative h-full overflow-hidden">
        <pre
          className="p-4 rounded-b-lg overflow-auto whitespace-pre h-full"
          style={{ margin: 0 }}
        >
          {highlightedCode}
        </pre>
        <textarea
          value={code}
          onChange={(e) => setCode(e.target.value)}
          spellCheck="false"
          className="absolute top-0 left-0 w-full h-full p-4 pl-12 bg-transparent text-transparent caret-white resize-none border-0 outline-none overflow-auto whitespace-pre font-mono text-sm leading-6"
        />
      </div>
    </div>
  );
};

/**
 * The dashboard for displaying and controlling classical parameters.
 */
const ParameterDashboard = ({ params, onParamChange }) => {
  return (
    <div className="bg-slate-800/50 p-4 rounded-lg border border-slate-700 h-full flex flex-col">
      <h3 className="text-base font-bold text-slate-200 mb-4 border-b border-slate-600 pb-2">
        Parameter Controls
      </h3>
      <div className="overflow-y-auto flex-grow pr-2">
        {Object.keys(params).length === 0 ? (
          <div className="text-center text-slate-400 pt-10">
            <p>
              Define parameters using{" "}
              <code className="bg-slate-700 p-1 rounded-md text-sm">
                (defparam 'name value)
              </code>
              .
            </p>
          </div>
        ) : (
          <div className="space-y-6">
            {Object.entries(params).map(([name, value]) => (
              <div key={name}>
                <label className="block text-sm font-medium text-slate-300 mb-2">
                  {name}
                </label>
                <div className="flex items-center space-x-4">
                  <input
                    type="range"
                    min="-6.28"
                    max="6.28"
                    step="0.01"
                    value={value}
                    onChange={(e) =>
                      onParamChange(name, parseFloat(e.target.value))
                    }
                    className="w-full h-2 bg-slate-700 rounded-lg appearance-none cursor-pointer"
                  />
                  <span className="font-mono text-indigo-300 w-20 text-center bg-slate-700 py-1 rounded-md">
                    {value.toFixed(2)}
                  </span>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};

/**
 * A visual representation of the quantum circuit.
 */
const QuantumCircuitVisualizer = ({ circuit }) => {
  const GATE_WIDTH = 40;
  const PADDING = 20;

  if (!circuit || circuit.numQubits === 0) {
    return (
      <div className="bg-slate-800/50 p-4 rounded-lg border border-slate-700 h-full flex flex-col">
        <h3 className="text-base font-bold text-slate-200 mb-4 border-b border-slate-600 pb-2">
          Circuit Visualizer
        </h3>
        <div className="text-center text-slate-400 pt-10 flex-grow">
          <p>Define a valid circuit in the editor to see a visualization.</p>
        </div>
      </div>
    );
  }

  const timeSlots = Array(circuit.numQubits).fill(0);
  const gatePositions = circuit.gates.map((gate) => {
    const involvedQubits = gate.qubits;
    const startTimeSlot = Math.max(...involvedQubits.map((q) => timeSlots[q]));
    const position = {
      x: startTimeSlot * (GATE_WIDTH + PADDING) + PADDING,
      gate,
    };
    const endTimeSlot = startTimeSlot + 1;
    involvedQubits.forEach((q) => (timeSlots[q] = endTimeSlot));
    return position;
  });

  const circuitWidth =
    Math.max(1, ...timeSlots) * (GATE_WIDTH + PADDING) + PADDING * 2;

  return (
    <div className="bg-slate-800/50 p-4 rounded-lg border border-slate-700 h-full flex flex-col">
      <h3 className="text-base font-bold text-slate-200 mb-4 border-b border-slate-600 pb-2">
        Circuit:{" "}
        <span className="font-mono text-indigo-300">{circuit.name}</span>
      </h3>
      <div className="overflow-x-auto overflow-y-hidden flex-grow relative">
        <svg width={circuitWidth} height="100%" className="min-h-[150px]">
          {Array.from({ length: circuit.numQubits }).map((_, i) => (
            <g key={`qubit-${i}`}>
              <line
                x1="0"
                y1={30 + i * 50}
                x2={circuitWidth}
                y2={30 + i * 50}
                stroke="#475569"
                strokeWidth="2"
              />
              <text
                x="5"
                y={35 + i * 50}
                fill="#94a3b8"
                fontSize="12"
                className="font-mono"
              >{`q${i}`}</text>
            </g>
          ))}
          {gatePositions.map(({ x, gate }, index) => {
            const isControlled =
              gate.type.startsWith("C") && gate.qubits.length > 1;
            const controlQubit = isControlled ? gate.qubits[0] : -1;
            const targetQubits = isControlled
              ? gate.qubits.slice(1)
              : gate.qubits;

            return (
              <g key={index}>
                {isControlled &&
                  targetQubits.map((targetQubit) => (
                    <line
                      key={`line-${targetQubit}`}
                      x1={x + GATE_WIDTH / 2}
                      y1={30 + controlQubit * 50}
                      x2={x + GATE_WIDTH / 2}
                      y2={30 + targetQubit * 50}
                      stroke="#818cf8"
                      strokeWidth="2"
                    />
                  ))}
                {isControlled && (
                  <circle
                    cx={x + GATE_WIDTH / 2}
                    cy={30 + controlQubit * 50}
                    r="5"
                    fill="#818cf8"
                  />
                )}
                {targetQubits.map((qubitIndex) => (
                  <g
                    key={`gate-${qubitIndex}`}
                    transform={`translate(${x}, ${10 + qubitIndex * 50})`}
                  >
                    <rect
                      width={GATE_WIDTH}
                      height={GATE_WIDTH}
                      rx="4"
                      fill={isControlled ? "#4338ca" : "#6366f1"}
                      stroke="#a5b4fc"
                    />
                    <text
                      x={GATE_WIDTH / 2}
                      y={GATE_WIDTH / 2 + 5}
                      textAnchor="middle"
                      fill="white"
                      fontSize="14"
                      fontWeight="bold"
                    >
                      {isControlled ? gate.type.substring(1) : gate.type}
                    </text>
                    {gate.args.some((arg) => arg.startsWith("'")) && (
                      <text
                        x={GATE_WIDTH / 2}
                        y={GATE_WIDTH + 12}
                        textAnchor="middle"
                        fill="#f78c6c"
                        fontSize="10"
                        className="font-mono"
                      >
                        {gate.args
                          .find((arg) => arg.startsWith("'"))
                          .substring(1)}
                      </text>
                    )}
                  </g>
                ))}
              </g>
            );
          })}
        </svg>
      </div>
    </div>
  );
};

/**
 * The panel for showing execution progress and output logs, now with result visualization.
 */
const ExecutionPanel = ({
  logs,
  result,
  onRun,
  onStop,
  isRunning,
  isMockMode,
}) => {
  const logEndRef = useRef(null);

  useEffect(() => {
    logEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [logs, result]);

  const basisStates = useMemo(() => {
    if (!result || !result.probabilities) return [];
    const numQubits = Math.log2(result.probabilities.length);
    return Array.from(
      { length: 1 << numQubits },
      (_, i) => `|${i.toString(2).padStart(numQubits, "0")}⟩`,
    );
  }, [result]);

  return (
    <div className="bg-slate-900 rounded-lg border border-slate-700 flex flex-col h-full">
      <div className="flex justify-between items-center p-3 border-b border-slate-700 flex-shrink-0">
        <div className="flex items-center gap-4">
          <h3 className="text-base font-bold text-slate-200">
            Execution & Output
          </h3>
          {isMockMode && (
            <span className="text-xs font-semibold text-yellow-500 bg-yellow-900/50 px-2 py-1 rounded-full">
              Demonstration Mode
            </span>
          )}
        </div>
        <div className="flex gap-2">
          <button
            onClick={onRun}
            disabled={isRunning}
            className="px-4 py-2 bg-indigo-600 rounded-md hover:bg-indigo-700 transition-colors font-semibold disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2"
          >
            {isRunning ? "Running..." : "Run"}
          </button>
          <button
            onClick={onStop}
            disabled={!isRunning}
            className="px-4 py-2 bg-red-600 rounded-md hover:bg-red-700 transition-colors font-semibold disabled:opacity-50 disabled:cursor-not-allowed"
          >
            Stop
          </button>
        </div>
      </div>
      <div className="p-4 font-mono text-sm text-slate-300 overflow-y-auto flex-grow">
        {logs.map((log, index) => (
          <div
            key={index}
            className="whitespace-pre-wrap"
            dangerouslySetInnerHTML={{ __html: log }}
          ></div>
        ))}
        {result && result.probabilities && (
          <div className="mt-4 pt-4 border-t border-slate-700">
            <h4 className="text-slate-200 font-bold mb-2">
              Simulation Results
            </h4>
            <div className="space-y-2 font-mono text-sm max-h-40 overflow-y-auto pr-2">
              {result.probabilities.map((prob, i) => (
                <div key={i} className="flex items-center gap-4">
                  <span className="text-cyan-300 w-24">{basisStates[i]}</span>
                  <div className="flex-grow bg-slate-700 rounded-full h-4">
                    <div
                      className="bg-cyan-500 h-4 rounded-full transition-all duration-500"
                      style={{ width: `${prob * 100}%` }}
                    />
                  </div>
                  <span className="w-20 text-right">
                    {(prob * 100).toFixed(2)}%
                  </span>
                </div>
              ))}
            </div>
          </div>
        )}
        {result && result.error && (
          <div className="mt-4 pt-4 border-t border-slate-700">
            <h4 className="text-red-400 font-bold mb-2">Simulation Error</h4>
            <p className="text-red-300">{result.error}</p>
          </div>
        )}
        <div ref={logEndRef} />
      </div>
    </div>
  );
};

// --- FIX: Create a mock simulator to be used as a fallback if WASM fails to load ---
const mockSimulator = {
  run_simulation: (jsonPayload) => {
    const payload = JSON.parse(jsonPayload);
    const numQubits = payload.numQubits || 1;
    const numOutcomes = 1 << numQubits;
    // Return a uniform distribution as a mock result
    const probabilities = Array(numOutcomes).fill(1 / numOutcomes);
    return JSON.stringify({ probabilities });
  },
};

/**
 * The main page for the QCL IDE, combining all components.
 */
const QclIdePage = () => {
  const initialCode = `
; Welcome to the Visual QCL IDE!
; This IDE now uses a real WASM-based quantum simulator.

; 1. Define classical parameters.
(defparam 'theta 1.57)
(defparam 'phi 3.14)

; 2. Define a quantum circuit.
(defcircuit 'bell_state (
    (H 0)
    (CX 0 1)
))

; 3. Run the circuit on the simulator.
; The output panel will show the final state probabilities.
(run 'bell_state)
    `.trim();

  const [code, setCode] = useState(initialCode);
  const [params, setParams] = useState({});
  const [circuit, setCircuit] = useState(null);
  const [logs, setLogs] = useState([]);
  const [simResult, setSimResult] = useState(null);
  const [isRunning, setIsRunning] = useState(false);

  const [wasm, setWasm] = useState(null);
  const [isMockMode, setIsMockMode] = useState(false);

  const debouncedCode = useDebounce(code, 300);

  // --- CORRECTED: Effect to load the WASM module with a graceful fallback ---
  useEffect(() => {
    const loadAndInitializeWasm = async () => {
      try {
        const wasmModule = await import("quantum-simulator-wasm");
        await wasmModule.default();
        setWasm(wasmModule);
        setLogs(["> Real quantum simulator loaded successfully."]);
      } catch (err) {
        console.error("Error loading and initializing WASM module:", err);
        setIsMockMode(true);
        // Do not add a scary error to the logs, the UI will indicate mock mode.
        setLogs([]);
      }
    };

    loadAndInitializeWasm();
  }, []);

  // Effect for parsing code and updating UI panels
  useEffect(() => {
    const newParams = parseParameters(debouncedCode);
    setParams(newParams);
    const newCircuit = parseCircuit(debouncedCode);
    setCircuit(newCircuit);
  }, [debouncedCode]);

  // Handler for UI -> Code binding
  const handleParamChange = (name, newValue) => {
    setParams((prev) => ({ ...prev, [name]: newValue }));
    const regex = new RegExp(`(\\(defparam\\s+'${name}'\\s+)([0-9.-]+)(\\))`);
    const newCode = code.replace(regex, `$1${newValue.toFixed(2)}$3`);
    setCode(newCode);
  };

  // --- UPDATED: Real execution logic with fallback ---
  const handleRun = () => {
    if (isRunning || !circuit) {
      if (!circuit)
        setLogs((prev) => [
          ...prev,
          '<span class="text-red-400">> No valid circuit defined to run.</span>',
        ]);
      return;
    }

    setIsRunning(true);
    setSimResult(null);

    // Use the real WASM module if loaded, otherwise use the mock simulator
    const simulator = wasm || mockSimulator;

    setLogs([
      `<span class="text-yellow-400">[Workflow]</span> Starting simulation...`,
    ]);

    // Translate our parsed circuit into the format the WASM simulator expects.
    const timeSlots = Array(circuit.numQubits).fill(0);
    const moments = [];

    circuit.gates.forEach((gate) => {
      const resolvedArgs = gate.args.map((arg) => {
        if (arg.startsWith("'")) {
          const paramName = arg.substring(1);
          return params[paramName] !== undefined ? params[paramName] : 0;
        }
        return isNaN(parseFloat(arg)) ? arg : parseFloat(arg);
      });

      const gateQubits = gate.qubits;
      const momentIndex = Math.max(0, ...gateQubits.map((q) => timeSlots[q]));

      while (moments.length <= momentIndex) {
        moments.push([]);
      }

      let simGate;
      // --- FIX: Handle CX gate from parser and map to CNOT for the simulator ---
      if (gate.type === "CX" || gate.type === "CNOT") {
        simGate = {
          type: "CNOT",
          control: gateQubits[0],
          target: gateQubits[1],
        };
      } else if (["RX", "RY", "RZ"].includes(gate.type)) {
        simGate = {
          type: gate.type,
          qubit: gateQubits[0],
          theta: resolvedArgs.find((a) => typeof a === "number"),
        };
      } else {
        simGate = { type: gate.type, qubit: gateQubits[0] };
      }
      moments[momentIndex].push(simGate);

      const endTimeSlot = momentIndex + 1;
      gateQubits.forEach((q) => (timeSlots[q] = endTimeSlot));
    });

    const circuitPayload = {
      numQubits: circuit.numQubits,
      moments: moments.filter((m) => m.length > 0),
    };

    setTimeout(() => {
      try {
        const resultJson = simulator.run_simulation(
          JSON.stringify(circuitPayload),
        );
        const result = JSON.parse(resultJson);
        setSimResult(result);
        setLogs((prev) => [
          ...prev,
          `<span class="text-green-400">[Workflow]</span> Simulation finished.`,
        ]);
      } catch (e) {
        console.error("Error running simulation:", e);
        setSimResult({ error: e.message });
      } finally {
        setIsRunning(false);
      }
    }, 100);
  };

  const handleStop = () => {
    setIsRunning(false);
  };

  return (
    <div className="p-4 h-full flex gap-4">
      <div className="w-3/5 h-full">
        <QclCodeEditor code={code} setCode={setCode} />
      </div>
      <div className="w-2/5 h-full flex flex-col gap-4">
        <div className="flex-1 min-h-0">
          <ParameterDashboard
            params={params}
            onParamChange={handleParamChange}
          />
        </div>
        <div className="flex-1 min-h-0">
          <QuantumCircuitVisualizer circuit={circuit} />
        </div>
        <div className="flex-1 min-h-0">
          <ExecutionPanel
            logs={logs}
            result={simResult}
            onRun={handleRun}
            onStop={handleStop}
            isRunning={isRunning}
            isMockMode={isMockMode}
          />
        </div>
      </div>
    </div>
  );
};

//================================================================================
// --- APP LAYOUT & ROUTER (Simplified for this example) ---
//================================================================================
export default function App() {
  return (
    <>
      <style>{`
            html, body, #root {
                height: 100%;
                overflow: hidden;
                background-color: #020617;
                color: #e2e8f0;
            }
            ::-webkit-scrollbar { width: 8px; height: 8px; }
            ::-webkit-scrollbar-track { background: #1e293b; border-radius: 4px; }
            ::-webkit-scrollbar-thumb { background: #475569; border-radius: 4px; }
            ::-webkit-scrollbar-thumb:hover { background: #64748b; }
        `}</style>
      <main className="h-full">
        <QclIdePage />
      </main>
    </>
  );
}
