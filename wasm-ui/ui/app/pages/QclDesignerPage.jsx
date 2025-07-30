import React, { useState, useCallback, useMemo, useEffect, useRef } from 'react';
import { HashRouter, Routes, Route, NavLink, Outlet } from 'react-router-dom';

// --- HELPER HOOKS & UTILS ---

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
  const regex = /\(defparam\s+'([a-zA-Z0-9_-]+)\s+([0-9.]+)\)/g;
  let match;
  while ((match = regex.exec(code)) !== null) {
    params[match[1]] = parseFloat(match[2]);
  }
  return params;
}

//================================================================================
// --- QCL IDE COMPONENTS ---
//================================================================================

/**
 * A simple code editor with syntax highlighting for our custom QCL language.
 * This version uses a robust tokenization approach to avoid the previous errors.
 */
const QclCodeEditor = ({ code, setCode }) => {
  const highlightedCode = useMemo(() => {
    // Define tokens and their corresponding regex patterns and colors.
    // The order is important: more specific tokens should come first.
    const tokenDefs = [
      { type: 'comment', regex: /;.*/, color: '#676e95' },
      { type: 'command', regex: /\(defparam|\(defcircuit|\(run|\(loop/, color: '#c792ea' },
      { type: 'symbol', regex: /'[a-zA-Z0-9_-]+/, color: '#f78c6c' },
      { type: 'number', regex: /[0-9.]+/, color: '#82aaff' },
      { type: 'paren', regex: /[()]/, color: '#e2e8f0' }, // Default color for parentheses
    ];

    // Combine all regexes into one, using capturing groups.
    const combinedRegex = new RegExp(tokenDefs.map(t => `(${t.regex.source})`).join('|'), 'g');

    // Split the code by the tokens. This will interleave plain text with the matched tokens.
    const parts = code.split(combinedRegex).filter(Boolean);

    return parts.map((part, index) => {
      // Find which token definition this part matches.
      for (const def of tokenDefs) {
        if (def.regex.test(part)) {
          // If it's a match, wrap it in a styled span.
          return <span key={index} style={{ color: def.color }}>{part}</span>;
        }
      }
      // If it's not a recognized token, render it as plain text.
      return <React.Fragment key={index}>{part}</React.Fragment>;
    });
  }, [code]);

  return (
      <div className="relative font-mono text-sm bg-slate-900 rounded-lg border border-slate-700 h-full">
            <pre
                className="p-4 rounded-lg overflow-auto whitespace-pre-wrap h-full"
                style={{ margin: 0 }}
            >
                {highlightedCode}
            </pre>
        <textarea
            value={code}
            onChange={(e) => setCode(e.target.value)}
            spellCheck="false"
            className="absolute top-0 left-0 w-full h-full p-4 bg-transparent text-transparent caret-white resize-none border-0 outline-none overflow-auto whitespace-pre-wrap font-mono text-sm leading-relaxed"
        />
      </div>
  );
};


/**
 * The dashboard for displaying and controlling classical parameters.
 */
const ParameterDashboard = ({ params, onParamChange }) => {
  return (
      <div className="bg-slate-800/50 p-4 rounded-lg border border-slate-700 h-full">
        <h3 className="text-lg font-bold text-slate-200 mb-4 border-b border-slate-600 pb-2">
          Parameters
        </h3>
        {Object.keys(params).length === 0 ? (
            <div className="text-center text-slate-400 pt-10">
              <p>Define parameters in the editor using <code className="bg-slate-700 p-1 rounded-md text-sm">(defparam 'name value)</code>.</p>
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
                          min="0"
                          max={2 * Math.PI}
                          step="0.01"
                          value={value}
                          onChange={(e) => onParamChange(name, parseFloat(e.target.value))}
                          className="w-full h-2 bg-slate-700 rounded-lg appearance-none cursor-pointer"
                      />
                      <span className="font-mono text-indigo-300 w-16 text-center bg-slate-700 py-1 rounded-md">
                                    {value.toFixed(2)}
                                </span>
                    </div>
                  </div>
              ))}
            </div>
        )}
      </div>
  );
};

/**
 * The panel for showing execution progress and output logs.
 */
const ExecutionPanel = ({ logs, onRun, isRunning }) => {
  const logEndRef = useRef(null);

  useEffect(() => {
    logEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [logs]);

  return (
      <div className="bg-slate-900 rounded-lg border border-slate-700 flex flex-col h-full">
        <div className="flex justify-between items-center p-3 border-b border-slate-700 flex-shrink-0">
          <h3 className="text-lg font-bold text-slate-200">Execution & Output</h3>
          <button
              onClick={onRun}
              disabled={isRunning}
              className="px-4 py-2 bg-indigo-600 rounded-md hover:bg-indigo-700 transition-colors font-semibold disabled:opacity-50 disabled:cursor-wait flex items-center gap-2"
          >
            {isRunning ? (
                <>
                  <svg className="animate-spin h-5 w-5" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                    <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"></circle>
                    <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                  </svg>
                  Running...
                </>
            ) : "Run Workflow"}
          </button>
        </div>
        <div className="p-4 font-mono text-sm text-slate-300 overflow-y-auto flex-grow">
          {logs.map((log, index) => (
              <div key={index} className="whitespace-pre-wrap">{log}</div>
          ))}
          <div ref={logEndRef} />
        </div>
      </div>
  );
};


/**
 * The main page for the QCL IDE, combining all components.
 */
const QclIdePage = () => {
  const initialCode = `
; Welcome to the QCL IDE!
; Define classical parameters that can be tuned.
(defparam 'theta 1.57)
(defparam 'phi 3.14)

; Define a quantum circuit using these parameters.
(defcircuit 'my_ansatz (
    (H 0)
    (RY 'theta 0)
    (CX 0 1)
    (RZ 'phi 1)
))

; Run the workflow.
(loop 5 (
    ; In a real scenario, you might update 'theta here.
    (run 'my_ansatz)
))
    `.trim();

  const [code, setCode] = useState(initialCode);
  const [params, setParams] = useState({});
  const [logs, setLogs] = useState(['> Ready. Press "Run Workflow" to start.']);
  const [isRunning, setIsRunning] = useState(false);

  const debouncedCode = useDebounce(code, 500);

  // Effect for two-way binding: Code -> Parameters Dashboard
  useEffect(() => {
    const newParams = parseParameters(debouncedCode);
    setParams(newParams);
  }, [debouncedCode]);

  // Handler for two-way binding: Parameters Dashboard -> Code
  const handleParamChange = (name, newValue) => {
    // Update the parameter state immediately for a responsive UI
    setParams(prev => ({ ...prev, [name]: newValue }));

    // Update the code
    const regex = new RegExp(`(\\(defparam\\s+'${name}'\\s+)([0-9.]+)(\\))`);
    if (regex.test(code)) {
      const newCode = code.replace(regex, `$1${newValue.toFixed(2)}$3`);
      setCode(newCode);
    }
  };

  // Mock execution logic
  const handleRun = () => {
    setIsRunning(true);
    setLogs(['> Starting workflow execution...']);

    const mockLogs = [
      "Defining parameter 'theta' = " + (params.theta?.toFixed(2) || 'N/A'),
      "Defining parameter 'phi' = " + (params.phi?.toFixed(2) || 'N/A'),
      "Defining circuit 'my_ansatz'...",
      "Entering Loop (5 iterations)...",
      ...Array.from({length: 5}, (_, i) => `  [Iter ${i+1}/5] Running circuit... Measured <z_obs> = ${(Math.random() * 2 - 1).toFixed(3)}`),
      "> Workflow finished successfully."
    ];

    let logIndex = 0;
    const interval = setInterval(() => {
      if (logIndex < mockLogs.length) {
        setLogs(prev => [...prev, mockLogs[logIndex]]);
        logIndex++;
      } else {
        clearInterval(interval);
        setIsRunning(false);
      }
    }, 400);
  };

  return (
      <div className="p-4 h-full flex gap-4">
        {/* Left Column */}
        <div className="flex flex-col gap-4 w-2/3">
          {/* Editor Panel */}
          <div className="h-3/5">
            <QclCodeEditor code={code} setCode={setCode} />
          </div>
          {/* Execution Panel */}
          <div className="h-2/5">
            <ExecutionPanel logs={logs} onRun={handleRun} isRunning={isRunning} />
          </div>
        </div>
        {/* Right Column */}
        <div className="w-1/3">
          <ParameterDashboard params={params} onParamChange={handleParamChange} />
        </div>
      </div>
  );
};


//================================================================================
// --- APP LAYOUT & ROUTER ---
//================================================================================

// This component ensures its children are only rendered on the client side.
const ClientOnly = ({ children }) => {
  const [hasMounted, setHasMounted] = useState(false);
  useEffect(() => { setHasMounted(true); }, []);
  if (!hasMounted) { return null; }
  return children;
};

const AppLayout = () => (
    <div className="flex flex-col h-screen bg-slate-950 text-slate-200">
      <header className="bg-slate-900/70 backdrop-blur-sm border-b border-slate-700/80 p-4 shadow-lg z-20 flex justify-between items-center shrink-0">
        <div className="flex items-center gap-8">
          <h1 className="text-xl font-bold text-white">Quantum UI</h1>
          <nav className="flex items-center gap-x-1 sm:gap-x-3 md:gap-x-6 text-xs sm:text-sm font-medium text-slate-400">
            <NavLink
                to="/"
                className={({ isActive }) =>
                    `py-2 px-1 md:px-2 nav-link ${isActive ? "active" : ""}`
                }
            >
              QCL IDE
            </NavLink>
          </nav>
        </div>
      </header>
      <div className="flex-1 overflow-y-auto relative">
        <Outlet />
      </div>
    </div>
);

export default function App() {
  return (
      <>
        <style>{`
        html, body, #root { height: 100%; }
        .nav-link {
            border-bottom: 2px solid transparent;
            transition: color 0.2s, border-color 0.2s;
            cursor: pointer;
        }
        .nav-link:hover {
            color: #e2e8f0; /* slate-200 */
        }
        .nav-link.active {
            color: #818cf8; /* indigo-400 */
            border-bottom-color: #818cf8; /* indigo-400 */
        }
        /* Custom scrollbar for webkit browsers */
        ::-webkit-scrollbar { width: 8px; }
        ::-webkit-scrollbar-track { background: #1e293b; }
        ::-webkit-scrollbar-thumb { background: #475569; border-radius: 4px; }
        ::-webkit-scrollbar-thumb:hover { background: #64748b; }
      `}</style>
        <ClientOnly>
          <HashRouter>
            <Routes>
              <Route path="/" element={<AppLayout />}>
                <Route index element={<QclIdePage />} />
              </Route>
            </Routes>
          </HashRouter>
        </ClientOnly>
      </>
  );
}
