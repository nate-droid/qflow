import React, { useState, useCallback, useMemo, useEffect } from 'react';

// --- Helper Components ---

// Icon for a specific gate
const GateIcon = ({ gate }) => {
    const gateStyles = {
        H: 'bg-yellow-500 border-yellow-600',
        X: 'bg-red-500 border-red-600',
        Y: 'bg-green-500 border-green-600',
        Z: 'bg-blue-500 border-blue-600',
    };
    const style = gateStyles[gate] || 'bg-gray-400 border-gray-500';
    return (
        <div className={`w-8 h-8 rounded-md flex items-center justify-center text-white font-bold text-sm border-b-2 ${style}`}>
            {gate}
        </div>
    );
};

// CNOT gate's control part
const CnotControl = () => (
    <div className="w-8 h-8 flex items-center justify-center">
        <div className="w-4 h-4 bg-blue-500 rounded-full border-2 border-blue-300"></div>
    </div>
);

// CNOT gate's target part
const CnotTarget = () => (
    <div className="w-8 h-8 flex items-center justify-center relative">
        <div className="w-8 h-8 border-2 border-blue-500 rounded-full flex items-center justify-center">
            <div className="w-0.5 h-5 bg-blue-500 absolute"></div>
            <div className="w-5 h-0.5 bg-blue-500 absolute"></div>
        </div>
    </div>
);

// --- Main Application Components ---

const GatePalette = ({ selectedGate, setSelectedGate, addQubit, removeQubit, numQubits, clearCircuit, onRun, isSimulating, wasmLoaded }) => {
    const gates = ['H', 'X', 'Y', 'Z', 'CNOT'];

    return (
        <div className="p-4 bg-gray-800 rounded-lg shadow-lg text-white">
            <h2 className="text-xl font-bold mb-4 text-center border-b border-gray-600 pb-2">Controls</h2>

            <div className="mb-6">
                <h3 className="font-semibold mb-2 text-center">Qubits</h3>
                <div className="flex items-center justify-center space-x-4 bg-gray-700 p-2 rounded-md">
                    <button onClick={removeQubit} className="px-3 py-1 bg-red-600 rounded-md hover:bg-red-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed" disabled={numQubits <= 1}>-</button>
                    <span className="font-mono text-lg">{numQubits}</span>
                    <button onClick={addQubit} className="px-3 py-1 bg-green-600 rounded-md hover:bg-green-700 transition-colors disabled:opacity-50" disabled={numQubits >= 8}>+</button>
                </div>
            </div>

            <div>
                <h3 className="font-semibold mb-3 text-center">Gates</h3>
                <div className="grid grid-cols-3 gap-3">
                    {gates.map(gate => (
                        <button
                            key={gate}
                            onClick={() => setSelectedGate(gate)}
                            className={`p-2 rounded-md transition-all duration-200 border-2 ${selectedGate === gate ? 'border-cyan-400 bg-gray-600' : 'border-transparent bg-gray-700 hover:bg-gray-600'}`}
                        >
                            {gate}
                        </button>
                    ))}
                </div>
            </div>

            <div className="mt-6 space-y-2">
                <button onClick={onRun} disabled={!wasmLoaded || isSimulating} className="w-full py-2 bg-green-600 rounded-md hover:bg-green-700 transition-colors font-semibold disabled:opacity-50 disabled:cursor-wait">
                    {isSimulating ? 'Simulating...' : (wasmLoaded ? 'Run Simulation' : 'Loading Engine...')}
                </button>
                <button onClick={clearCircuit} className="w-full py-2 bg-indigo-600 rounded-md hover:bg-indigo-700 transition-colors font-semibold">
                    Clear Circuit
                </button>
            </div>
        </div>
    );
};

const CircuitGrid = ({ numQubits, moments, setMoments, selectedGate, setSelectedGate }) => {
    const [cnotState, setCnotState] = useState({ isConnecting: false, controlQubit: null, momentIndex: null });

    const handleCellClick = (qubitIndex, momentIndex) => {
        // --- CNOT Gate Logic ---
        if (selectedGate === 'CNOT') {
            if (!cnotState.isConnecting) {
                setCnotState({ isConnecting: true, controlQubit: qubitIndex, momentIndex });
            } else {
                if (cnotState.momentIndex === momentIndex && cnotState.controlQubit !== qubitIndex) {
                    const newMoments = [...moments];
                    if (!newMoments[momentIndex]) newMoments[momentIndex] = [];

                    const gateAtControl = newMoments[momentIndex].find(g => g.qubit === cnotState.controlQubit || (g.type === 'CNOT' && (g.control === cnotState.controlQubit || g.target === cnotState.controlQubit)));
                    const gateAtTarget = newMoments[momentIndex].find(g => g.qubit === qubitIndex || (g.type === 'CNOT' && (g.control === qubitIndex || g.target === qubitIndex)));

                    if(!gateAtControl && !gateAtTarget) {
                        newMoments[momentIndex].push({ type: 'CNOT', control: cnotState.controlQubit, target: qubitIndex });
                        setMoments(newMoments);
                    } else {
                        console.warn("Cannot place CNOT over an existing gate.");
                    }
                }
                setCnotState({ isConnecting: false, controlQubit: null, momentIndex: null });
                setSelectedGate(null);
            }
        }
        // --- Single Qubit Gate Logic ---
        else if (selectedGate) {
            const newMoments = [...moments];
            if (!newMoments[momentIndex]) newMoments[momentIndex] = [];

            const existingGateIndex = newMoments[momentIndex].findIndex(g => g.qubit === qubitIndex || (g.type === 'CNOT' && (g.control === qubitIndex || g.target === qubitIndex)));

            if (existingGateIndex !== -1) {
                const existingGate = newMoments[momentIndex][existingGateIndex];
                if (existingGate.type === selectedGate) {
                    newMoments[momentIndex].splice(existingGateIndex, 1);
                } else {
                    newMoments[momentIndex][existingGateIndex] = { type: selectedGate, qubit: qubitIndex };
                }
            } else {
                newMoments[momentIndex].push({ type: selectedGate, qubit: qubitIndex });
            }
            setMoments(newMoments);
        }
    };

    const numMoments = 20;

    const gridCells = useMemo(() => {
        const cells = [];
        for (let q = 0; q < numQubits; q++) {
            for (let m = 0; m < numMoments; m++) {
                const moment = moments[m] || [];
                let gateComponent = null;

                const singleQubitGate = moment.find(g => g.qubit === q);
                const cnotGate = moment.find(g => g.type === 'CNOT' && (g.control === q || g.target === q));

                if (singleQubitGate) {
                    gateComponent = <GateIcon gate={singleQubitGate.type} />;
                } else if (cnotGate) {
                    if (cnotGate.control === q) {
                        gateComponent = <CnotControl />;
                    } else if (cnotGate.target === q) {
                        gateComponent = <CnotTarget />;
                    }
                }

                const isPendingCnotControl = cnotState.isConnecting && cnotState.controlQubit === q && cnotState.momentIndex === m;

                cells.push(
                    <div
                        key={`${q}-${m}`}
                        className={`w-12 h-12 flex items-center justify-center rounded-md transition-colors duration-200 cursor-pointer ${isPendingCnotControl ? 'bg-blue-500/30' : 'hover:bg-gray-700/50'}`}
                        onClick={() => handleCellClick(q, m)}
                    >
                        {gateComponent}
                    </div>
                );
            }
        }
        return cells;
    }, [numQubits, moments, cnotState, selectedGate]);

    const cnotLines = useMemo(() => {
        const lines = [];
        moments.forEach((moment, momentIndex) => {
            if (!moment) return;
            moment.forEach((gate, gateIndex) => {
                if (gate.type === 'CNOT') {
                    const topQubit = Math.min(gate.control, gate.target);
                    const bottomQubit = Math.max(gate.control, gate.target);
                    const height = (bottomQubit - topQubit) * 48;

                    lines.push(
                        <div
                            key={`${momentIndex}-${gateIndex}`}
                            className="absolute bg-blue-500 w-0.5"
                            style={{
                                left: `${momentIndex * 48 + 23.5}px`,
                                top: `${topQubit * 48 + 24}px`,
                                height: `${height}px`,
                            }}
                        />
                    );
                }
            });
        });
        return lines;
    }, [moments]);

    return (
        <div className="flex-grow p-4 bg-gray-900 rounded-lg overflow-x-auto">
            <div className="relative inline-block" style={{ minWidth: `${numMoments * 48}px` }}>
                <div className="absolute inset-0 z-0">{cnotLines}</div>
                <div
                    className="relative z-10 grid"
                    style={{
                        gridTemplateColumns: `repeat(${numMoments}, 48px)`,
                        gridTemplateRows: `repeat(${numQubits}, 48px)`,
                    }}
                >
                    {Array.from({ length: numQubits }).map((_, qIndex) => (
                        <div key={`line-${qIndex}`} className="absolute h-0.5 bg-gray-500" style={{ top: `${qIndex * 48 + 23.5}px`, left: '16px', right: '16px', zIndex: -1 }} />
                    ))}
                    {gridCells}
                </div>
                {Array.from({ length: numQubits }).map((_, qIndex) => (
                    <div key={`label-${qIndex}`} className="absolute text-gray-400 font-mono text-sm" style={{ left: '-40px', top: `${qIndex * 48 + 14}px` }}>
                        q{qIndex}:
                    </div>
                ))}
            </div>
        </div>
    );
};

const SimulationOutput = ({ result, numQubits, isSimulating }) => {
    if (isSimulating) {
        return (
            <div className="mt-6 p-4 bg-gray-800 rounded-lg shadow-lg text-white text-center">
                <p className="text-lg font-semibold animate-pulse">Simulating...</p>
            </div>
        );
    }

    if (!result) {
        return (
            <div className="mt-6 p-4 bg-gray-800 rounded-lg shadow-lg text-white text-center">
                <p className="text-gray-400">Run a simulation to see the results.</p>
            </div>
        );
    }

    if (result.error) {
        return (
            <div className="mt-6 p-4 bg-red-900 border border-red-700 rounded-lg shadow-lg text-white">
                <h3 className="text-lg font-bold text-red-300">Simulation Error</h3>
                <p className="font-mono text-sm mt-2 text-red-200">{result.error}</p>
            </div>
        );
    }

    const basisStates = Array.from({ length: 1 << numQubits }, (_, i) =>
        `|${i.toString(2).padStart(numQubits, '0')}⟩`
    );

    return (
        <div className="mt-6 p-4 bg-gray-800 rounded-lg shadow-lg text-white">
            <h3 className="text-xl font-bold mb-4 text-center">Simulation Results</h3>
            <div className="space-y-2 font-mono text-sm max-h-60 overflow-y-auto pr-2">
                {result.probabilities.map((prob, i) => (
                    <div key={i} className="flex items-center gap-4">
                        <span className="text-cyan-300">{basisStates[i]}</span>
                        <div className="flex-grow bg-gray-700 rounded-full h-4">
                            <div
                                className="bg-cyan-500 h-4 rounded-full transition-all duration-500"
                                style={{ width: `${prob * 100}%` }}
                            />
                        </div>
                        <span className="w-20 text-right">{(prob * 100).toFixed(2)}%</span>
                    </div>
                ))}
            </div>
        </div>
    );
};


// --- App Component ---
export default function App() {
    const [numQubits, setNumQubits] = useState(3);
    const [moments, setMoments] = useState([]);
    const [selectedGate, setSelectedGate] = useState(null);
    const [wasm, setWasm] = useState(null);
    const [simResult, setSimResult] = useState(null);
    const [isSimulating, setIsSimulating] = useState(false);

    // Effect to load the WASM module
    useEffect(() => {
        const loadWasm = async () => {
            try {
                // In a real app, this would be: const wasmModule = await import("quantum-simulator-wasm");
                // For this demo, we mock the module.
                const loadWasm = async () => {
                    try {
                        const wasmModule = await import("quantum-simulator-wasm");
                        setWasm(wasmModule);
                    } catch (err) {
                        console.error("Error loading WASM module:", err);
                        // You could set an error state here to show in the UI
                    }
                };
                loadWasm();

            } catch (err) {
                console.error("Error loading WASM module:", err);
            }
        };
        loadWasm();
    }, []);

    const clearCircuit = useCallback(() => {
        setMoments([]);
        setSelectedGate(null);
        setSimResult(null);
    }, []);

    const handleQubitChange = (newNumQubits) => {
        if (newNumQubits < numQubits) {
            const newMoments = moments.map(moment =>
                moment.filter(gate => {
                    if (gate.type === 'CNOT') {
                        return gate.control < newNumQubits && gate.target < newNumQubits;
                    }
                    return gate.qubit < newNumQubits;
                })
            ).filter(moment => moment && moment.length > 0);
            setMoments(newMoments);
        }
        setNumQubits(newNumQubits);
        setSimResult(null); // Results are invalid when qubits change
    };

    const handleRunSimulation = () => {
        if (!wasm || isSimulating) return;
        setIsSimulating(true);
        setSimResult(null);

        const circuitPayload = {
            numQubits: numQubits,
            moments: moments.filter(m => m && m.length > 0)
        };
        const circuitJson = JSON.stringify(circuitPayload);

        // Use a timeout to allow the UI to update to the "loading" state
        setTimeout(() => {
            try {
                const resultJson = wasm.run_simulation(circuitJson);
                const result = JSON.parse(resultJson);
                setSimResult(result);
            } catch (e) {
                console.error("Error running simulation:", e);
                setSimResult({ error: e.message });
            } finally {
                setIsSimulating(false);
            }
        }, 100);
    };

    return (
        <div className="bg-gray-900 text-white min-h-screen flex flex-col items-center justify-center p-4 font-sans">
            <div className="w-full max-w-7xl mx-auto">
                <header className="text-center mb-6">
                    <h1 className="text-4xl font-bold text-cyan-300">Quantum Circuit Simulator</h1>
                    <p className="text-gray-400 mt-2">Design a circuit, then press "Run Simulation" to see the final state probabilities.</p>
                </header>

                <main className="flex flex-col md:flex-row gap-6">
                    <aside className="w-full md:w-64 flex-shrink-0">
                        <GatePalette
                            selectedGate={selectedGate}
                            setSelectedGate={setSelectedGate}
                            addQubit={() => handleQubitChange(numQubits + 1)}
                            removeQubit={() => handleQubitChange(numQubits - 1)}
                            numQubits={numQubits}
                            clearCircuit={clearCircuit}
                            onRun={handleRunSimulation}
                            isSimulating={isSimulating}
                            wasmLoaded={!!wasm}
                        />
                    </aside>

                    <div className="flex-grow flex flex-col">
                        <div className="bg-gray-800 p-6 rounded-lg shadow-lg" style={{'--num-qubits': numQubits}}>
                            <CircuitGrid
                                numQubits={numQubits}
                                moments={moments}
                                setMoments={setMoments}
                                selectedGate={selectedGate}
                                setSelectedGate={setSelectedGate}
                            />
                        </div>
                        <SimulationOutput result={simResult} numQubits={numQubits} isSimulating={isSimulating} />
                    </div>
                </main>
            </div>
        </div>
    );
}
