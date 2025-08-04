import React, { useState, useMemo, useEffect, useRef } from "react";

// --- Helper Hooks & Utils ---

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

function parseParameters(code) {
  const params = {};
  const regex = /\(defparam\s+([a-zA-Z0-9_-]+)\s+([0-9.-]+)\)/g;
  let match;
  while ((match = regex.exec(code)) !== null) {
    params[match[1]] = parseFloat(match[2]);
  }
  return params;
}

function parseCircuit(code) {
  // Updated regex to match: (defcircuit name (params...) (body...))
  const circuitRegex =
      /\(defcircuit\s+([a-zA-Z0-9_-]+)\s+\(([\s\S]*?)\)\s+\(([\s\S]*)\)\s*\)/;
  const circuitMatch = code.match(circuitRegex);

  if (!circuitMatch) return null;

  const name = circuitMatch[1];
  const paramsRaw = circuitMatch[2];
  const body = circuitMatch[3];

  // DEBUG: Print body string before gate parsing
  console.log("DEBUG parseCircuit body string:", JSON.stringify(body));

  // Parse params as array of names
  const params = paramsRaw
      .split(/\s+/)
      .map((p) => p.trim())
      .filter((p) => p.length > 0);

  // Parse gates from body (allow for extra parentheses around gate list)
  const gateRegex = /\(\s*\(?\s*([A-Z]+)\s+((?:[a-zA-Z0-9_.-]+\s*)+)\)?\s*\)/g;
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
    params,
    gates,
    numQubits: maxQubit > -1 ? maxQubit + 1 : 0,
  };
}

//================================================================================
// --- Reusable Circuit Components from CircuitSimulatorPage.jsx ---
//================================================================================

const GateIcon = ({ gate, theta }) => {
  const gateStyles = {
    H: "bg-yellow-500 border-yellow-600",
    X: "bg-red-500 border-red-600",
    CX: "bg-blue-500 border-blue-600",
    CNOT: "bg-blue-500 border-blue-600",
    RX: "bg-purple-500 border-purple-600",
  };
  const style = gateStyles[gate] || "bg-gray-400 border-gray-500";
  return (
      <div
          className={`w-10 h-10 rounded-md flex items-center justify-center text-white font-bold text-xs border-b-2 ${style} flex-col`}
          title={theta !== undefined ? `${gate}(${theta})` : gate}
      >
        <span>{gate}</span>
      </div>
  );
};

const CnotControl = () => (
    <div className="w-10 h-10 flex items-center justify-center">
      <div className="w-4 h-4 bg-blue-500 rounded-full border-2 border-blue-300"></div>
    </div>
);

const CnotTarget = () => (
    <div className="w-10 h-10 flex items-center justify-center relative">
      <div className="w-8 h-8 border-2 border-blue-500 rounded-full flex items-center justify-center">
        <div className="w-0.5 h-5 bg-blue-500 absolute"></div>
        <div className="w-5 h-0.5 bg-blue-500 absolute"></div>
      </div>
    </div>
);

//================================================================================
// --- QCL IDE COMPONENTS ---
//================================================================================

const QclCodeEditor = ({ code, setCode, errorLine }) => {
  const editorRef = useRef(null);
  const backdropRef = useRef(null);

  const handleScroll = (e) => {
    if (backdropRef.current) {
      backdropRef.current.scrollTop = e.target.scrollTop;
      backdropRef.current.scrollLeft = e.target.scrollLeft;
    }
  };

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
      const parts = [];
      let lastIndex = 0;

      for (const match of line.matchAll(combinedRegex)) {
        const tokenIndex = match.index;
        if (tokenIndex > lastIndex) {
          parts.push(line.substring(lastIndex, tokenIndex));
        }
        const tokenValue = match[0];
        let matchedDef = null;
        for (let i = 1; i < match.length; i++) {
          if (match[i] !== undefined) {
            matchedDef = tokenDefs[i - 1];
            break;
          }
        }
        if (matchedDef && matchedDef.color) {
          parts.push(
              <span
                  key={`${lineIndex}-${lastIndex}`}
                  style={{ color: matchedDef.color }}
              >
              {tokenValue}
            </span>,
          );
        } else {
          parts.push(tokenValue);
        }
        lastIndex = tokenIndex + tokenValue.length;
      }
      if (lastIndex < line.length) {
        parts.push(line.substring(lastIndex));
      }

      return (
          <div
              key={lineIndex}
              className={`line leading-6 ${lineIndex + 1 === errorLine ? "bg-red-900/30" : ""}`}
          >
          <span className="w-8 inline-block text-slate-600 text-right pr-4 select-none">
            {lineIndex + 1}
          </span>
            {parts.map((part, partIndex) => (
                <React.Fragment key={partIndex}>{part}</React.Fragment>
            ))}
          </div>
      );
    });
  }, [code, errorLine]);

  return (
      <div className="relative font-mono text-sm bg-slate-900 rounded-lg border border-slate-700 h-full flex flex-col">
        <div className="p-3 border-b border-slate-700 flex-shrink-0">
          <h3 className="text-base font-bold text-slate-200">QCL Code Editor</h3>
        </div>
        <div className="relative flex-1 overflow-hidden">
        <pre
            ref={backdropRef}
            className="absolute inset-0 p-4 pl-12 whitespace-pre font-mono text-sm leading-6 pointer-events-none overflow-auto"
            style={{ margin: 0 }}
        >
          {highlightedCode}
        </pre>
          <textarea
              ref={editorRef}
              value={code}
              onChange={(e) => setCode(e.target.value)}
              onScroll={handleScroll}
              spellCheck="false"
              className="absolute inset-0 p-4 pl-12 bg-transparent text-transparent caret-white resize-none border-0 outline-none overflow-auto whitespace-pre font-mono text-sm leading-6"
          />
        </div>
      </div>
  );
};

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
                    (defparam name value)
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

const mockSimulator = {
  run_simulation: (jsonPayload) => {
    const payload = JSON.parse(jsonPayload);
    const numQubits = payload.numQubits || 1;
    const numOutcomes = 1 << numQubits;
    const probabilities = Array(numOutcomes).fill(1 / numOutcomes);
    return JSON.stringify({ probabilities });
  },
};

const GatePalette = ({ onDragStart }) => {
  const gates = ["H", "X", "CX", "RX"];
  return (
      <div className="bg-slate-800/50 p-4 rounded-lg border border-slate-700">
        <h3 className="text-base font-bold text-slate-200 mb-4 border-b border-slate-600 pb-2">
          Gate Palette
        </h3>
        <div className="grid grid-cols-4 gap-2">
          {gates.map((gate) => (
              <div
                  key={gate}
                  draggable
                  onDragStart={(e) => onDragStart(e, gate)}
                  className="flex justify-center items-center p-1 cursor-grab active:cursor-grabbing"
              >
                <GateIcon gate={gate} />
              </div>
          ))}
        </div>
      </div>
  );
};

const CircuitGrid = ({ circuit, onCircuitUpdate }) => {
  // Helper to export QCL code for the current circuit (new syntax)
  const exportQclCode = () => {
    if (!circuit) return "";
    const paramsString =
        circuit.params && circuit.params.length > 0
            ? circuit.params.join(" ")
            : "";
    const gatesString = circuit.gates
        .map((gate) => `    (${gate.type} ${gate.args.join(" ")})`)
        .join("\n");
    return `(defcircuit ${circuit.name} (${paramsString}) (\n${gatesString}\n))`;
  };
  const numQubits = circuit ? circuit.numQubits : 1;
  const numMoments = 20;
  const cellHeight = 64;
  const cellWidth = 56;

  const moments = useMemo(() => {
    if (!circuit) return [];
    const gridMoments = Array(numMoments)
        .fill(null)
        .map(() => []);
    const timeSlots = Array(numQubits).fill(0);

    circuit.gates.forEach((gate) => {
      const involvedQubits = gate.qubits;
      if (involvedQubits.length === 0) return;

      const momentIndex = Math.max(
          0,
          ...involvedQubits.map((q) => timeSlots[q]),
      );
      if (momentIndex >= numMoments) return;

      if (gate.type === "CX" || gate.type === "CNOT") {
        gridMoments[momentIndex].push({
          type: "CNOT",
          control: gate.qubits[0],
          target: gate.qubits[1],
        });
      } else {
        gridMoments[momentIndex].push({
          type: gate.type,
          qubit: gate.qubits[0],
          args: gate.args,
        });
      }

      const endTimeSlot = momentIndex + 1;
      involvedQubits.forEach((q) => (timeSlots[q] = endTimeSlot));
    });
    return gridMoments;
  }, [circuit]);

  const handleDrop = (e, qubitIndex, momentIndex) => {
    e.preventDefault();
    const gateType = e.dataTransfer.getData("gateType");
    if (!gateType || !circuit) return;

    const newGates = [...circuit.gates];

    if (gateType === "CX") {
      if (qubitIndex > 0) {
        newGates.push({
          type: "CX",
          args: [`${qubitIndex - 1}`, `${qubitIndex}`],
          qubits: [qubitIndex - 1, qubitIndex],
        });
      }
    } else {
      newGates.push({
        type: gateType,
        args: [`${qubitIndex}`],
        qubits: [qubitIndex],
      });
    }

    onCircuitUpdate({ ...circuit, gates: newGates });
  };

  const handleDragOver = (e) => {
    e.preventDefault();
  };

  if (!circuit) {
    return (
        <div className="bg-slate-800/50 p-4 rounded-lg border border-slate-700 h-full flex flex-col">
          <h3 className="text-base font-bold text-slate-200 mb-4 border-b border-slate-600 pb-2">
            Circuit Designer
          </h3>
          <div className="text-center text-slate-400 pt-10 flex-grow">
            <p>Define a circuit in the editor to enable the designer.</p>
          </div>
        </div>
    );
  }

  return (
      <div className="bg-slate-800/50 p-4 rounded-lg border border-slate-700 h-full flex flex-col">
        <h3 className="text-base font-bold text-slate-200 mb-4 border-b border-slate-600 pb-2">
          Circuit Designer:{" "}
          <span className="font-mono text-indigo-300">{circuit.name}</span>
        </h3>

        <div className="overflow-auto">
          <div
              className="relative inline-block"
              style={{ minWidth: `${numMoments * cellWidth}px` }}
          >
            {Array.from({ length: numQubits }).map((_, qIndex) => (
                <div
                    key={`line-${qIndex}`}
                    className="absolute h-0.5 bg-gray-500"
                    style={{
                      top: `${qIndex * cellHeight + 31.5}px`,
                      left: "16px",
                      right: "16px",
                      zIndex: 0,
                    }}
                />
            ))}
            <div
                className="relative z-10 grid"
                style={{
                  gridTemplateColumns: `repeat(${numMoments}, ${cellWidth}px)`,
                  gridTemplateRows: `repeat(${numQubits}, ${cellHeight}px)`,
                }}
            >
              {Array.from({ length: numQubits * numMoments }).map((_, i) => {
                const q = Math.floor(i / numMoments);
                const m = i % numMoments;
                const momentGates = moments[m] || [];
                const singleGate = momentGates.find((g) => g.qubit === q);
                const cnotGate = momentGates.find(
                    (g) => g.type === "CNOT" && (g.control === q || g.target === q),
                );
                let gateComponent = null;

                if (singleGate) {
                  gateComponent = <GateIcon gate={singleGate.type} />;
                } else if (cnotGate) {
                  if (cnotGate.control === q) gateComponent = <CnotControl />;
                  else gateComponent = <CnotTarget />;
                }

                return (
                    <div
                        key={`${q}-${m}`}
                        onDrop={(e) => handleDrop(e, q, m)}
                        onDragOver={handleDragOver}
                        className="w-full h-full flex items-center justify-center border-r border-dashed border-slate-700/50"
                    >
                      {gateComponent}
                    </div>
                );
              })}
            </div>
          </div>
        </div>
      </div>
  );
};

/**
 * The main page for the QCL IDE, combining all components.
 */
const QclIdePage = () => {
  const initialCode = `
; Welcome to the Visual QCL IDE!
; This example demonstrates a VQE-like optimization loop.
; Press "Run" to watch theta update automatically.

(defparam theta 1.57)

(defcircuit vqe_ansatz (theta) (
    ((RX theta 0))
))

(loop 20 (
    (let theta) ; In a real scenario, an optimizer would update this.
    (run vqe_ansatz)
))
    `.trim();

  const [code, setCode] = useState(initialCode);
  const [params, setParams] = useState({});
  const [circuit, setCircuit] = useState(null);
  const [logs, setLogs] = useState([]);
  const [simResult, setSimResult] = useState(null);
  const [isRunning, setIsRunning] = useState(false);

  // DEBUG: Log parseCircuit output
  useEffect(() => {
    const parsed = parseCircuit(code);
    console.log("DEBUG parseCircuit output:", parsed);
  }, [code]);

  const [wasm, setWasm] = useState(null);
  const [isMockMode, setIsMockMode] = useState(false);
  const loopIntervalRef = useRef(null);
  const thetaRef = useRef(0);

  const debouncedCode = useDebounce(code, 500);

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
        setLogs([]);
      }
    };
    loadAndInitializeWasm();
  }, []);

  useEffect(() => {
    const newParams = parseParameters(debouncedCode);
    setParams(newParams);
    const newCircuit = parseCircuit(debouncedCode);
    setCircuit(newCircuit);
  }, [debouncedCode]);

  const handleCircuitUpdateFromGrid = (updatedCircuit) => {
    setCircuit(updatedCircuit);

    const gatesString = updatedCircuit.gates
        .map((gate) => {
          return `    (${gate.type} ${gate.args.join(" ")})`;
        })
        .join("\n");

    // Build params string for circuit
    const paramsString =
        updatedCircuit.params && updatedCircuit.params.length > 0
            ? updatedCircuit.params.join(" ")
            : "";

    // Replace old circuit definition with new syntax
    const newCode = code.replace(
        /\(defcircuit\s+[a-zA-Z0-9_-]+\s+\([^\)]*\)\s+\([^\)]*\)\s*\)/,
        `(defcircuit ${updatedCircuit.name} (${paramsString}) (\n${gatesString}\n))`,
    );
    setCode(newCode);
  };

  const updateParameterValue = (paramName, value) => {
    setParams((prev) => ({ ...prev, [paramName]: value }));
    setCode((currentCode) => {
      const regex = new RegExp(
          `(\\(defparam\\s+${paramName}\\s+)([0-9.-]+)(\\))`,
      );
      return currentCode.replace(regex, `$1${value.toFixed(2)}$3`);
    });
  };

  const runSingleSimulation = (currentParams) => {
    const simulator = wasm || mockSimulator;
    if (!circuit) return;

    const timeSlots = Array(circuit.numQubits).fill(0);
    const moments = [];

    circuit.gates.forEach((gate) => {
      const resolvedArgs = gate.args.map((arg) => {
        if (arg.startsWith("'")) {
          const paramName = arg.substring(1);
          return currentParams[paramName] !== undefined
              ? currentParams[paramName]
              : 0;
        }
        return isNaN(parseFloat(arg)) ? arg : parseFloat(arg);
      });

      const gateQubits = gate.qubits;
      const momentIndex = Math.max(0, ...gateQubits.map((q) => timeSlots[q]));

      while (moments.length <= momentIndex) {
        moments.push([]);
      }

      let simGate;
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

    try {
      const resultJson = simulator.run_simulation(
          JSON.stringify(circuitPayload),
      );
      return JSON.parse(resultJson);
    } catch (e) {
      console.error("Error running simulation:", e);
      return { error: e.message };
    }
  };

  const handleRun = () => {
    if (isRunning || !circuit) return;

    const loopMatch = code.match(/\(loop\s+([0-9]+)/);

    if (loopMatch) {
      const iterations = parseInt(loopMatch[1], 10);
      let currentIteration = 0;

      thetaRef.current = params.theta;

      setIsRunning(true);
      setLogs([
        `<span class="text-yellow-400">[Workflow]</span> Starting loop for ${iterations} iterations...`,
      ]);

      loopIntervalRef.current = setInterval(() => {
        if (currentIteration >= iterations) {
          handleStop();
          return;
        }

        const result = runSingleSimulation({
          ...params,
          theta: thetaRef.current,
        });
        setSimResult(result);

        const energy = result.probabilities
            ? (result.probabilities[0] || 0) - (result.probabilities[1] || 0)
            : 1;
        const gradient = -Math.sin(thetaRef.current);
        const newTheta = thetaRef.current - 0.4 * gradient;

        setLogs((prev) => [
          ...prev,
          `[Iter ${currentIteration + 1}/${iterations}] Energy: ${energy.toFixed(3)}, New Theta: ${newTheta.toFixed(3)}`,
        ]);

        updateParameterValue("theta", newTheta);
        thetaRef.current = newTheta;
        currentIteration++;
      }, 700);
    } else {
      setIsRunning(true);
      setSimResult(null);
      setLogs([
        `<span class="text-yellow-400">[Workflow]</span> Starting single simulation...`,
      ]);
      setTimeout(() => {
        const result = runSingleSimulation(params);
        setSimResult(result);
        setLogs((prev) => [
          ...prev,
          `<span class="text-green-400">[Workflow]</span> Simulation finished.`,
        ]);
        setIsRunning(false);
      }, 100);
    }
  };

  const handleStop = () => {
    if (loopIntervalRef.current) {
      clearInterval(loopIntervalRef.current);
      loopIntervalRef.current = null;
    }
    setIsRunning(false);
    setLogs((prev) => [
      ...prev,
      `<span class="text-yellow-400">[Workflow]</span> Loop finished or stopped.`,
    ]);
  };

  const handleDragStart = (e, gateType) => {
    e.dataTransfer.setData("gateType", gateType);
  };

  return (
      <div className="p-4 h-full flex gap-4">
        <div className="w-3/5 h-full flex flex-col gap-4">
          <div className="flex-1 min-h-0">
            <QclCodeEditor code={code} setCode={setCode} />
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
        <div className="w-2/5 h-full flex flex-col gap-4">
          <div className="flex-1 min-h-0">
            <CircuitGrid
                circuit={circuit}
                onCircuitUpdate={handleCircuitUpdateFromGrid}
            />
          </div>
          <div>
            <GatePalette onDragStart={handleDragStart} />
          </div>
          <div className="flex-1 min-h-0">
            <ParameterDashboard
                params={params}
                onParamChange={updateParameterValue}
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
