import React, { useState, useMemo, useEffect, useRef } from "react";
// REMOVED: All Recharts imports and dependencies.

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
  const regex = /\(defparam\s+'?([a-zA-Z0-9_-]+)'?\s+([0-9.-]+)\)/g;
  let match;
  while ((match = regex.exec(code)) !== null) {
    params[match[1]] = parseFloat(match[2]);
  }
  return params;
}

function parseCircuit(code) {
  const circuitRegex =
      /\(defcircuit\s+'?([a-zA-Z0-9_-]+)'?\s+\(([\s\S]*?)\)\s+\(([\s\S]*)\)\s*\)/;
  const circuitMatch = code.match(circuitRegex);

  if (!circuitMatch) return null;

  const name = circuitMatch[1];
  const paramsRaw = circuitMatch[2];
  const body = circuitMatch[3];

  const params = paramsRaw
      .split(/\s+/)
      .map((p) => p.trim().replace(/'/g, ''))
      .filter((p) => p.length > 0);

  const gateRegex = /\(\s*([A-Z]+)\s+((?:'?\w+\s*)+)\)/g;
  let gateMatch;
  const gates = [];
  let maxQubit = -1;

  while ((gateMatch = gateRegex.exec(body)) !== null) {
    const type = gateMatch[1];
    const args = gateMatch[2].trim().split(/\s+/).map(arg => arg.replace(/'/g, ''));
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
// --- Reusable Circuit Components ---
//================================================================================

const GateIcon = ({ gate, theta }) => {
  const gateStyles = {
    H: "bg-yellow-500 border-yellow-600",
    X: "bg-red-500 border-red-600",
    CX: "bg-blue-500 border-blue-600",
    CNOT: "bg-blue-500 border-blue-600",
    RX: "bg-purple-500 border-purple-600",
    RY: "bg-purple-500 border-purple-600",
    RZ: "bg-purple-500 border-purple-600",
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
        regex: /\b(defparam|defcircuit|run-circuit|loop|let|write-file|cond|let\*)\b/g,
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

      const matches = Array.from(line.matchAll(combinedRegex));

      for (const match of matches) {
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
              autoCapitalize="off"
              autoComplete="off"
              autoCorrect="off"
              className="absolute inset-0 p-4 pl-12 bg-transparent text-transparent caret-white resize-none border-0 outline-none overflow-auto whitespace-pre font-mono text-sm leading-6"
          />
        </div>
      </div>
  );
};

const ParameterDashboard = ({ params, onParamChange, isRunning }) => {
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
                            disabled={isRunning}
                            onChange={(e) =>
                                onParamChange(name, parseFloat(e.target.value))
                            }
                            className="w-full h-2 bg-slate-700 rounded-lg appearance-none cursor-pointer disabled:opacity-50"
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

// --- State Vector Display Component ---
const StateVectorDisplay = ({ stateVector, basisStates }) => {
  return (
      <div className="space-y-2 font-mono text-sm max-h-40 overflow-y-auto pr-2">
        {stateVector.map((amp, i) => (
            <div key={i} className="flex items-center gap-4">
              <span className="text-cyan-300 w-24">{basisStates[i]}</span>
              <div className="flex-grow text-right">
                <span className="text-slate-300">{amp.re.toFixed(4)}</span>
                <span className="text-purple-400">{amp.im >= 0 ? ' + ' : ' - '}{Math.abs(amp.im).toFixed(4)}i</span>
              </div>
            </div>
        ))}
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
  const [activeTab, setActiveTab] = useState('probabilities');

  useEffect(() => {
    logEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [logs, result]);

  const basisStates = useMemo(() => {
    if (!result || (!result.probabilities && !result.state_vector)) return [];
    const numQubits = Math.log2(result.probabilities?.length || result.state_vector?.length || 0);
    if (numQubits % 1 !== 0) return [];
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
        <div className="p-4 font-mono text-sm text-slate-300 flex-grow flex flex-col overflow-hidden">
          <div className="flex-grow overflow-y-auto">
            {logs.map((log, index) => (
                <div
                    key={index}
                    className="whitespace-pre-wrap"
                    dangerouslySetInnerHTML={{ __html: log }}
                ></div>
            ))}
            <div ref={logEndRef} />
          </div>

          {result && (
              <div className="flex-shrink-0 mt-4 pt-4 border-t border-slate-700">
                <div className="flex border-b border-slate-700 mb-2">
                  <button onClick={() => setActiveTab('probabilities')} className={`px-4 py-2 text-sm font-medium ${activeTab === 'probabilities' ? 'border-b-2 border-indigo-500 text-white' : 'text-slate-400'}`}>Probabilities</button>
                  <button onClick={() => setActiveTab('state_vector')} className={`px-4 py-2 text-sm font-medium ${activeTab === 'state_vector' ? 'border-b-2 border-indigo-500 text-white' : 'text-slate-400'}`}>State Vector</button>
                </div>

                {activeTab === 'probabilities' && result.probabilities && (
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
                )}

                {activeTab === 'state_vector' && result.state_vector && (
                    <StateVectorDisplay stateVector={result.state_vector} basisStates={basisStates} />
                )}
              </div>
          )}
          {result && result.error && (
              <div className="flex-shrink-0 mt-4 pt-4 border-t border-slate-700">
                <h4 className="text-red-400 font-bold mb-2">Simulation Error</h4>
                <p className="text-red-300">{result.error}</p>
              </div>
          )}
        </div>
      </div>
  );
};

const mockSimulator = {
  run_simulation: (jsonPayload) => {
    try {
      const payload = JSON.parse(jsonPayload);
      const numQubits = payload.numQubits || 1;
      const numOutcomes = 1 << numQubits;

      let theta = 0;
      if (payload.moments) {
        for (const moment of payload.moments) {
          for (const gate of moment) {
            if (gate.theta !== undefined) {
              theta = gate.theta;
            }
          }
        }
      }

      // FIX: Create a more realistic state vector for multi-qubit systems
      const state_vector = Array.from({ length: numOutcomes }, () => ({re: 0, im: 0}));

      if (numQubits === 1) {
        state_vector[0] = { re: Math.cos(theta/2), im: 0 };
        state_vector[1] = { re: Math.sin(theta/2), im: 0 };
      } else if (numQubits === 2) {
        // Create a Bell state modulated by theta
        state_vector[0] = { re: Math.cos(theta/2), im: 0 };
        state_vector[3] = { re: Math.sin(theta/2), im: 0 };
      } else {
        // Default for > 2 qubits, just the ground state
        state_vector[0] = { re: 1, im: 0 };
      }

      const probabilities = state_vector.map(amp => amp.re * amp.re + amp.im * amp.im);

      return JSON.stringify({ probabilities, state_vector });
    } catch (e) {
      return JSON.stringify({ error: "Failed to parse simulation payload." });
    }
  },
};

const GatePalette = ({ onDragStart }) => {
  const gates = ["H", "X", "CNOT", "RX", "RY", "RZ"];
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

      let momentIndex = 0;
      for (const q of involvedQubits) {
        if (timeSlots[q] > momentIndex) {
          momentIndex = timeSlots[q];
        }
      }

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

    if (gateType === "CNOT") {
      // For simplicity, CNOT is added between the drop target and the qubit above.
      // A more robust implementation would allow selecting both control and target.
      if (qubitIndex > 0) {
        newGates.push({
          type: "CNOT",
          args: [`${qubitIndex - 1}`, `${qubitIndex}`],
          qubits: [qubitIndex - 1, qubitIndex],
        });
      }
    } else if (["RX", "RY", "RZ"].includes(gateType)) {
      // Parameterized gates need a parameter. We'll add a placeholder.
      // A real implementation might open a dialog to select the parameter.
      const paramName = circuit.params[0] || 'theta';
      newGates.push({
        type: gateType,
        args: [`'${paramName}`, `${qubitIndex}`],
        qubits: [qubitIndex]
      });
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
            <p className="text-sm mt-2">Example: <code className="bg-slate-700 p-1 rounded-md">(defcircuit 'name ('p) (...))</code></p>
          </div>
        </div>
    );
  }

  return (
      <div className="bg-slate-800/50 p-4 rounded-lg border border-slate-700 h-full flex flex-col">
        <div className="flex justify-between items-center mb-4 border-b border-slate-600 pb-2">
          <h3 className="text-base font-bold text-slate-200">
            Circuit Designer:{" "}
            <span className="font-mono text-indigo-300">{circuit.name}</span>
          </h3>
          <span className="text-sm text-slate-400">{circuit.numQubits} Qubits</span>
        </div>

        <div className="overflow-auto">
          <div
              className="relative inline-block"
              style={{ minWidth: `${numMoments * cellWidth}px` }}
          >
            {Array.from({ length: numQubits }).map((_, qIndex) => (
                <div key={`line-container-${qIndex}`} className="flex items-center" style={{height: `${cellHeight}px`}}>
                  <span className="text-xs text-slate-400 w-8 text-center font-mono">q{qIndex}</span>
                  <div
                      key={`line-${qIndex}`}
                      className="absolute h-0.5 bg-gray-500"
                      style={{
                        top: `${qIndex * cellHeight + (cellHeight/2) - 1}px`,
                        left: "40px",
                        right: "16px",
                        zIndex: 0,
                      }}
                  />
                </div>
            ))}
            <div
                className="absolute top-0 z-10 grid"
                style={{
                  gridTemplateColumns: `repeat(${numMoments}, ${cellWidth}px)`,
                  gridTemplateRows: `repeat(${numQubits}, ${cellHeight}px)`,
                  left: '40px'
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

// --- Optimizer Controls Component ---
const OptimizerControls = ({ optimizerConfig, setOptimizerConfig, isRunning }) => {
  return (
      <div className="bg-slate-800/50 p-4 rounded-lg border border-slate-700">
        <h3 className="text-base font-bold text-slate-200 mb-4 border-b border-slate-600 pb-2">
          Optimizer Settings
        </h3>
        <div className="space-y-4">
          <div>
            <label className="block text-sm font-medium text-slate-300 mb-1">
              Algorithm
            </label>
            <select
                value={optimizerConfig.algorithm}
                disabled={isRunning}
                onChange={(e) => setOptimizerConfig(c => ({...c, algorithm: e.target.value}))}
                className="w-full bg-slate-700 border border-slate-600 rounded-md p-2 text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500 disabled:opacity-50"
            >
              <option value="gradient_descent">Gradient Descent</option>
            </select>
          </div>
          <div>
            <label htmlFor="learning-rate" className="block text-sm font-medium text-slate-300 mb-1">
              Learning Rate ({optimizerConfig.learningRate})
            </label>
            <input
                id="learning-rate"
                type="range"
                min="0.01"
                max="1.0"
                step="0.01"
                disabled={isRunning}
                value={optimizerConfig.learningRate}
                onChange={(e) => setOptimizerConfig(c => ({...c, learningRate: parseFloat(e.target.value)}))}
                className="w-full h-2 bg-slate-700 rounded-lg appearance-none cursor-pointer disabled:opacity-50"
            />
          </div>
        </div>
      </div>
  );
};

// --- Custom SVG Chart Component ---
const OptimizationChart = ({ history }) => {
  if (history.length < 2) {
    return null;
  }

  const width = 400;
  const height = 200;
  const margin = { top: 20, right: 20, bottom: 30, left: 40 };

  const data = history.map(d => d.energy);
  const min = Math.min(...data);
  const max = Math.max(...data);

  const x = (i) => margin.left + (i / (data.length - 1)) * (width - margin.left - margin.right);
  const y = (value) => height - margin.bottom - ((value - min) / (max - min)) * (height - margin.top - margin.bottom);

  const path = data.map((d, i) => `${i === 0 ? 'M' : 'L'} ${x(i)} ${y(d)}`).join(' ');

  return (
      <div className="bg-slate-900 rounded-lg border border-slate-700 p-4 h-full flex flex-col">
        <h3 className="text-base font-bold text-slate-200 mb-4">
          Optimization Progress
        </h3>
        <div className="flex-grow flex items-center justify-center">
          <svg viewBox={`0 0 ${width} ${height}`} className="w-full h-full">
            {/* Y-axis */}
            <line x1={margin.left} y1={margin.top} x2={margin.left} y2={height - margin.bottom} stroke="#475569" />
            <text x={margin.left - 10} y={margin.top + 5} fill="#9ca3af" fontSize="10" textAnchor="end">{max.toFixed(2)}</text>
            <text x={margin.left - 10} y={height - margin.bottom} fill="#9ca3af" fontSize="10" textAnchor="end">{min.toFixed(2)}</text>

            {/* X-axis */}
            <line x1={margin.left} y1={height - margin.bottom} x2={width - margin.right} y2={height - margin.bottom} stroke="#475569" />
            <text x={margin.left} y={height - margin.bottom + 15} fill="#9ca3af" fontSize="10" textAnchor="start">1</text>
            <text x={width - margin.right} y={height - margin.bottom + 15} fill="#9ca3af" fontSize="10" textAnchor="end">{data.length}</text>

            {/* Line path */}
            <path d={path} stroke="#8884d8" strokeWidth="2" fill="none" />
          </svg>
        </div>
      </div>
  );
}


/**
 * The main page for the QCL IDE, combining all components.
 */
const QclIdePage = () => {
  const initialCode = `
; Welcome to the Visual QCL IDE!
; This example demonstrates a VQE-like optimization loop.
; 1. Define a parameter to optimize.
; 2. Define a circuit that uses it.
; 3. Use 'loop' to run the optimization.
; Press "Run" to watch the chart and parameters update.

(defparam 'theta 1.57)

(defcircuit 'vqe_ansatz ('theta) (
  (RX 'theta 0)
  (H 1)
  (CNOT 0 1)
))

(loop 50 (
  (run-circuit (vqe_ansatz 'theta))
))
    `.trim();

  const [code, setCode] = useState(initialCode);
  const [params, setParams] = useState({});
  const [circuit, setCircuit] = useState(null);
  const [logs, setLogs] = useState([]);
  const [simResult, setSimResult] = useState(null);
  const [isRunning, setIsRunning] = useState(false);
  const [isMockMode, setIsMockMode] = useState(true); // Default to mock mode
  const [wasm, setWasm] = useState(null);

  // --- State for optimizer and history ---
  const [optimizerConfig, setOptimizerConfig] = useState({
    algorithm: 'gradient_descent',
    learningRate: 0.4
  });
  const [optimizationHistory, setOptimizationHistory] = useState([]);

  const loopIntervalRef = useRef(null);
  const paramStateRef = useRef({});

  const debouncedCode = useDebounce(code, 500);

  // This effect attempts to load WASM but falls back to mock mode
  useEffect(() => {
    const loadWasm = async () => {
      try {
        // This will fail in this environment, triggering the catch block
        const wasmModule = await import("quantum-simulator-wasm");
        await wasmModule.default();
        setWasm(wasmModule);
        setIsMockMode(false);
        setLogs(prev => [...prev, "> Real quantum simulator loaded."]);
      } catch (err) {
        setIsMockMode(true);
        setLogs(prev => [...prev, "> Using mock simulator. Real engine not found."]);
      }
    };
    loadWasm();
  }, []);

  useEffect(() => {
    const newParams = parseParameters(debouncedCode);
    setParams(newParams);
    paramStateRef.current = newParams;
    const newCircuit = parseCircuit(debouncedCode);
    setCircuit(newCircuit);
  }, [debouncedCode]);

  const handleCircuitUpdateFromGrid = (updatedCircuit) => {
    setCircuit(updatedCircuit);

    const paramsString = updatedCircuit.params.join(" ");
    const gatesString = updatedCircuit.gates
        .map(gate => `  (${gate.type} ${gate.args.join(" ")})`)
        .join("\n");

    const newCode = code.replace(
        /\(defcircuit\s+'?\w+'?\s+\([\s\S]*?\)\s+\([\s\S]*?\)\s*\)/,
        `(defcircuit '${updatedCircuit.name} (${paramsString}) (\n${gatesString}\n))`
    );
    setCode(newCode);
  };

  const updateParameterValue = (paramName, value) => {
    const newParams = { ...paramStateRef.current, [paramName]: value };
    paramStateRef.current = newParams;
    setParams(newParams); // Update UI

    setCode((currentCode) => {
      const regex = new RegExp(
          `(\\(defparam\\s+'${paramName}'\\s+)([0-9.-]+)(\\))`,
      );
      return currentCode.replace(regex, `$1${value.toFixed(4)}$3`);
    });
  };

  const runSingleSimulation = (currentParams) => {
    const simulator = wasm || mockSimulator;
    if (!circuit) return { error: "No valid circuit defined." };

    const resolvedCircuit = {
      numQubits: circuit.numQubits,
      moments: []
    };

    const timeSlots = Array(circuit.numQubits).fill(0);

    for (const gate of circuit.gates) {
      const resolvedArgs = gate.args.map(arg => {
        if (currentParams[arg] !== undefined) {
          return currentParams[arg];
        }
        return isNaN(parseFloat(arg)) ? arg : parseFloat(arg);
      });

      const gateQubits = gate.qubits;
      let momentIndex = 0;
      for (const q of gateQubits) {
        if (timeSlots[q] > momentIndex) {
          momentIndex = timeSlots[q];
        }
      }

      while (resolvedCircuit.moments.length <= momentIndex) {
        resolvedCircuit.moments.push([]);
      }

      let simGate;
      if (gate.type === "CNOT") {
        simGate = { type: "CNOT", control: gateQubits[0], target: gateQubits[1] };
      } else if (["RX", "RY", "RZ"].includes(gate.type)) {
        simGate = { type: gate.type, qubit: gateQubits[0], theta: resolvedArgs.find(a => typeof a === 'number') };
      } else {
        simGate = { type: gate.type, qubit: gateQubits[0] };
      }
      resolvedCircuit.moments[momentIndex].push(simGate);

      const endTimeSlot = momentIndex + 1;
      gateQubits.forEach(q => timeSlots[q] = endTimeSlot);
    }

    try {
      const resultJson = simulator.run_simulation(JSON.stringify(resolvedCircuit));
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

      paramStateRef.current = parseParameters(code);

      setIsRunning(true);
      setLogs([
        `<span class="text-yellow-400">[Workflow]</span> Starting optimization loop for ${iterations} iterations...`,
        `<span class="text-yellow-400">[Optimizer]</span> Algorithm: ${optimizerConfig.algorithm}, LR: ${optimizerConfig.learningRate}`
      ]);
      setOptimizationHistory([]);

      loopIntervalRef.current = setInterval(() => {
        if (currentIteration >= iterations) {
          handleStop();
          return;
        }

        const result = runSingleSimulation(paramStateRef.current);

        if (result.error) {
          setLogs(prev => [...prev, `<span class="text-red-400">[Error]</span> ${result.error}`]);
          handleStop();
          return;
        }

        // Simple cost function: E = <Z> = P(0) - P(1)
        const energy = (result.probabilities[0] || 0) - (result.probabilities[1] || 0);

        setOptimizationHistory(prev => [...prev, { iteration: currentIteration + 1, energy: energy }]);

        // Gradient Descent Logic
        if (optimizerConfig.algorithm === 'gradient_descent') {
          Object.keys(paramStateRef.current).forEach(paramName => {
            // Approximate gradient using parameter-shift rule for RX gates
            // grad(E) = (E(theta + pi/2) - E(theta - pi/2)) / 2
            const paramValue = paramStateRef.current[paramName];

            const forwardParams = {...paramStateRef.current, [paramName]: paramValue + Math.PI / 2};
            const forwardResult = runSingleSimulation(forwardParams);
            const energyForward = (forwardResult.probabilities[0] || 0) - (forwardResult.probabilities[1] || 0);

            const backwardParams = {...paramStateRef.current, [paramName]: paramValue - Math.PI / 2};
            const backwardResult = runSingleSimulation(backwardParams);
            const energyBackward = (backwardResult.probabilities[0] || 0) - (backwardResult.probabilities[1] || 0);

            const gradient = (energyForward - energyBackward) / 2;

            const newParamValue = paramValue - optimizerConfig.learningRate * gradient;

            setLogs((prev) => [
              ...prev.slice(0, 50), // Keep log history from growing too large
              `[Iter ${currentIteration + 1}] Param: ${paramName}, E: ${energy.toFixed(4)}, Grad: ${gradient.toFixed(4)}, New: ${newParamValue.toFixed(4)}`,
            ]);

            updateParameterValue(paramName, newParamValue);
          });
        }

        currentIteration++;
      }, 500);
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
    const finalParams = paramStateRef.current;
    const finalResult = runSingleSimulation(finalParams);
    setSimResult(finalResult);
    setLogs((prev) => [
      ...prev,
      `<span class="text-yellow-400">[Workflow]</span> Loop finished or stopped.`,
    ]);
  };

  const handleDragStart = (e, gateType) => {
    e.dataTransfer.setData("gateType", gateType);
  };

  return (
      <div className="p-4 h-full grid grid-cols-12 gap-4">
        {/* Column 1: Code Editor & Execution Panel */}
        <div className="col-span-5 h-full flex flex-col gap-4">
          <div className="h-3/5">
            <QclCodeEditor code={code} setCode={setCode} />
          </div>
          <div className="h-2/5">
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

        {/* Column 2: Circuit Designer & Palette */}
        <div className="col-span-4 h-full flex flex-col gap-4">
          <div className="flex-grow min-h-0">
            <CircuitGrid
                circuit={circuit}
                onCircuitUpdate={handleCircuitUpdateFromGrid}
            />
          </div>
          <div className="flex-shrink-0">
            <GatePalette onDragStart={handleDragStart} />
          </div>
        </div>

        {/* Column 3: Controls & Results */}
        <div className="col-span-3 h-full flex flex-col gap-4">
          <div className="flex-shrink-0">
            <OptimizerControls
                optimizerConfig={optimizerConfig}
                setOptimizerConfig={setOptimizerConfig}
                isRunning={isRunning}
            />
          </div>
          <div className="flex-shrink-0 h-[240px]">
            <OptimizationChart history={optimizationHistory} />
          </div>
          <div className="flex-grow min-h-0">
            <ParameterDashboard
                params={params}
                onParamChange={updateParameterValue}
                isRunning={isRunning}
            />
          </div>
        </div>
      </div>
  );
};

//================================================================================
// --- APP LAYOUT & ROUTER ---
//================================================================================
export default function App() {
  return (
      <>
        <style>{`
            /* Using a more modern font and ensuring full height */
            @import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500;700&display=swap');
            html, body, #root {
                height: 100%;
                overflow: hidden;
                background-color: #020617; /* slate-950 */
                color: #e2e8f0; /* slate-200 */
                font-family: 'Inter', sans-serif;
            }
            /* Custom scrollbar styling */
            ::-webkit-scrollbar { width: 8px; height: 8px; }
            ::-webkit-scrollbar-track { background: #1e293b; border-radius: 4px; } /* slate-800 */
            ::-webkit-scrollbar-thumb { background: #475569; border-radius: 4px; } /* slate-600 */
            ::-webkit-scrollbar-thumb:hover { background: #64748b; } /* slate-500 */
        `}</style>
        <main className="h-full">
          <QclIdePage />
        </main>
      </>
  );
}
