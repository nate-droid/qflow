import React, { useState, useCallback, useMemo } from 'react';

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

const GatePalette = ({ selectedGate, setSelectedGate, addQubit, removeQubit, numQubits, clearCircuit }) => {
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

            <div className="mt-6">
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
                // Start CNOT connection
                setCnotState({ isConnecting: true, controlQubit: qubitIndex, momentIndex });
            } else {
                // Finish CNOT connection
                if (cnotState.momentIndex === momentIndex && cnotState.controlQubit !== qubitIndex) {
                    const newMoments = [...moments];
                    // Ensure the moment exists
                    if (!newMoments[momentIndex]) newMoments[momentIndex] = [];

                    // Check if a gate already exists at control or target
                    const gateAtControl = newMoments[momentIndex].find(g => g.qubit === cnotState.controlQubit || (g.type === 'CNOT' && (g.control === cnotState.controlQubit || g.target === cnotState.controlQubit)));
                    const gateAtTarget = newMoments[momentIndex].find(g => g.qubit === qubitIndex || (g.type === 'CNOT' && (g.control === qubitIndex || g.target === qubitIndex)));

                    if(!gateAtControl && !gateAtTarget) {
                        newMoments[momentIndex].push({
                            type: 'CNOT',
                            control: cnotState.controlQubit,
                            target: qubitIndex,
                        });
                        setMoments(newMoments);
                    } else {
                        console.warn("Cannot place CNOT over an existing gate.");
                    }
                }
                // Reset CNOT state regardless of success
                setCnotState({ isConnecting: false, controlQubit: null, momentIndex: null });
                setSelectedGate(null); // Deselect CNOT after placing
            }
        }
        // --- Single Qubit Gate Logic ---
        else if (selectedGate) {
            const newMoments = [...moments];
            if (!newMoments[momentIndex]) newMoments[momentIndex] = [];

            // Check if a gate already exists
            const existingGateIndex = newMoments[momentIndex].findIndex(g => g.qubit === qubitIndex || (g.type === 'CNOT' && (g.control === qubitIndex || g.target === qubitIndex)));

            if (existingGateIndex !== -1) {
                // If same gate is clicked, remove it. Otherwise, replace it.
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

    const numMoments = 20; // Fixed number of columns for simplicity

    // Memoize the grid rendering to prevent unnecessary re-renders
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

    // Memoize CNOT connection lines
    const cnotLines = useMemo(() => {
        const lines = [];
        moments.forEach((moment, momentIndex) => {
            if (!moment) return;
            moment.forEach((gate, gateIndex) => {
                if (gate.type === 'CNOT') {
                    const topQubit = Math.min(gate.control, gate.target);
                    const bottomQubit = Math.max(gate.control, gate.target);
                    const height = (bottomQubit - topQubit) * 48; // 48px is h-12

                    lines.push(
                        <div
                            key={`${momentIndex}-${gateIndex}`}
                            className="absolute bg-blue-500 w-0.5"
                            style={{
                                left: `${momentIndex * 48 + 23.5}px`, // 48px is w-12, 23.5 is for centering
                                top: `${topQubit * 48 + 24}px`, // 24 is for centering
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
                {/* Render CNOT connection lines underneath the grid */}
                <div className="absolute inset-0 z-0">
                    {cnotLines}
                </div>

                {/* The main grid for qubit lines and gates */}
                <div
                    className="relative z-10 grid"
                    style={{
                        gridTemplateColumns: `repeat(${numMoments}, 48px)`,
                        gridTemplateRows: `repeat(${numQubits}, 48px)`,
                    }}
                >
                    {/* Render Qubit Lines */}
                    {Array.from({ length: numQubits }).map((_, qIndex) => (
                        <div
                            key={`line-${qIndex}`}
                            className="absolute h-0.5 bg-gray-500"
                            style={{
                                top: `${qIndex * 48 + 23.5}px`,
                                left: '16px',
                                right: '16px',
                                zIndex: -1, // Place behind gates
                            }}
                        />
                    ))}
                    {gridCells}
                </div>

                {/* Qubit Labels */}
                {Array.from({ length: numQubits }).map((_, qIndex) => (
                    <div
                        key={`label-${qIndex}`}
                        className="absolute text-gray-400 font-mono text-sm"
                        style={{
                            left: '-40px',
                            top: `${qIndex * 48 + 14}px`, // 14px to vertically center
                        }}
                    >
                        q{qIndex}:
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

    const addQubit = () => setNumQubits(q => Math.min(q + 1, 8));
    const removeQubit = () => setNumQubits(q => Math.max(q - 1, 1));

    const clearCircuit = useCallback(() => {
        setMoments([]);
        setSelectedGate(null);
    }, []);

    // Adjust circuit if a qubit is removed that has gates on it
    const handleQubitChange = (newNumQubits) => {
        if (newNumQubits < numQubits) {
            const newMoments = moments.map(moment =>
                moment.filter(gate => {
                    if (gate.type === 'CNOT') {
                        return gate.control < newNumQubits && gate.target < newNumQubits;
                    }
                    return gate.qubit < newNumQubits;
                })
            ).filter(moment => moment.length > 0);
            setMoments(newMoments);
        }
        setNumQubits(newNumQubits);
    };

    return (
        <div className="bg-gray-900 text-white min-h-screen flex flex-col items-center justify-center p-4 font-sans">
            <div className="w-full max-w-7xl mx-auto">
                <header className="text-center mb-6">
                    <h1 className="text-4xl font-bold text-cyan-300">Quantum Circuit Simulator</h1>
                    <p className="text-gray-400 mt-2">Click a gate, then click on the grid to place it. For CNOT, click control then target.</p>
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
                        />
                    </aside>

                    <div className="flex-grow bg-gray-800 p-6 rounded-lg shadow-lg" style={{'--num-qubits': numQubits}}>
                        <CircuitGrid
                            numQubits={numQubits}
                            moments={moments}
                            setMoments={setMoments}
                            selectedGate={selectedGate}
                            setSelectedGate={setSelectedGate}
                        />
                    </div>
                </main>
            </div>
        </div>
    );
}
