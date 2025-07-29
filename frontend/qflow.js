document.addEventListener('DOMContentLoaded', function () {
    // --- Globals ---
    let currentWorkflow = null;
    let pollingTimeoutId = null;
    let svmPollingIntervalId = null;
    const API_BASE_URL = 'http://localhost:3000';

    // --- DOM Elements ---
    const dom = {
        nav: {
            svm: document.getElementById('nav-svm'),
            visualizer: document.getElementById('nav-visualizer'),
            qcbm: document.getElementById('nav-qcbm'),
            submitter: document.getElementById('nav-submitter'),
        },
        pages: {
            svm: document.getElementById('svm-page'),
            visualizer: document.getElementById('visualizer-page'),
            qcbm: document.getElementById('qcbm-submitter-page'),
            submitter: document.getElementById('qasm-submitter-page'),
        },
        pollingStatus: document.getElementById('polling-status'),
        // SVM Page Elements
        svm: {
            runButton: document.getElementById('run-experiment-button'),
            statusDisplay: document.getElementById('svm-status-display'),
            statusText: document.getElementById('svm-status-text'),
            errorDisplay: document.getElementById('svm-error-display'),
            errorText: document.getElementById('svm-error-text'),
            resultsArea: document.getElementById('results-area'),
            resultPlot: document.getElementById('result-plot'),
            resultMetrics: document.getElementById('result-metrics'),
            // Form Inputs
            dsGenerator: document.getElementById('ds-generator'),
            dsSamples: document.getElementById('ds-samples'),
            dsNoise: document.getElementById('ds-noise'),
            dsTestSize: document.getElementById('ds-test-size'),
            kernelImage: document.getElementById('kernel-image'),
            trainerC: document.getElementById('trainer-c'),
            outputModelName: document.getElementById('output-model-name'),
            outputPlotName: document.getElementById('output-plot-name'),
        },
        // Visualizer Page Elements
        visualizer: {
            graphOverlay: document.getElementById('graph-overlay'),
            fetchButton: document.getElementById('fetch-button'),
            workflowInput: document.getElementById('workflow-input'),
            namespaceInput: document.getElementById('namespace-input'),
            workflowNameDisplay: document.getElementById('workflow-name-display'),
            details: {
                content: document.getElementById('details-content'),
                placeholder: document.getElementById('details-placeholder'),
                fetchPrompt: document.getElementById('details-fetch-prompt'),
                data: document.getElementById('details-data'),
                title: document.getElementById('details-title'),
                status: document.getElementById('details-status'),
                circuitTitle: document.getElementById('details-circuit-title'),
                circuit: document.getElementById('details-circuit'),
                params: document.getElementById('details-params'),
                resultsBtnContainer: document.getElementById('results-button-container'),
            }
        },
        // QCBM Submitter Elements
        qcbm: {
            notification: document.getElementById('qcbm-notification'),
            workflowName: document.getElementById('qcbm-workflow-name'),
            namespace: document.getElementById('qcbm-namespace'),
            taskName: document.getElementById('qcbm-task-name'),
            image: document.getElementById('qcbm-image'),
            ansatz: document.getElementById('qcbm-ansatz'),
            trainingData: document.getElementById('qcbm-training-data'),
            optName: document.getElementById('qcbm-opt-name'),
            optEpochs: document.getElementById('qcbm-opt-epochs'),
            optLr: document.getElementById('qcbm-opt-lr'),
            optParams: document.getElementById('qcbm-opt-params'),
            submitButton: document.getElementById('qcbm-submit-button'),
        },
        // QASM Submitter Elements
        qasm: {
            notification: document.getElementById('qasm-notification'),
            nameInput: document.getElementById('qasm-workflow-name'),
            namespaceInput: document.getElementById('qasm-namespace'),
            input: document.getElementById('qasm-input'),
            submitButton: document.getElementById('qasm-submit-button'),
        },
        ml: {
            runButton: document.getElementById('ml-run-button'),
            metrics: document.getElementById('ml-metrics'),
            plot: document.getElementById('ml-plot'),
            resultsArea: document.getElementById('ml-results-area'),
            errorDisplay: document.getElementById('ml-error-display'),
        },
        // Shared Modals
        errorModal: {
            overlay: document.getElementById('error-modal-overlay'),
            content: document.getElementById('error-modal-content'),
        },
        resultsModal: {
            overlay: document.getElementById('results-modal-overlay'),
            title: document.getElementById('results-modal-title'),
            logContainer: document.getElementById('raw-log-container'),
            log: document.getElementById('results-modal-log'),
            visContainer: document.getElementById('q-state-vis-container'),
            vis: document.getElementById('q-state-vis'),
            tabVis: document.getElementById('tab-vis'),
            tabLog: document.getElementById('tab-log'),
        }
    };

    // --- Page Navigation ---
    function showPage(pageName) {
        Object.values(dom.pages).forEach(p => p.classList.add('hidden'));
        Object.values(dom.nav).forEach(n => n.classList.remove('active'));

        if (dom.pages[pageName]) {
            dom.pages[pageName].classList.remove('hidden');
            dom.nav[pageName].classList.add('active');
        } else {
            dom.pages.visualizer.classList.remove('hidden');
            dom.nav.visualizer.classList.add('active');
        }
    }

    dom.nav.svm.addEventListener('click', () => showPage('svm'));
    dom.nav.visualizer.addEventListener('click', () => showPage('visualizer'));
    dom.nav.qcbm.addEventListener('click', () => showPage('qcbm'));
    dom.nav.submitter.addEventListener('click', () => showPage('submitter'));
    dom.nav.ml = document.getElementById('nav-ml');
    dom.pages.ml = document.getElementById('ml-page');
    dom.nav.ml.addEventListener('click', () => showPage('ml'));

    // --- Modal Logic ---
    function showModal(modal, title, content) {
        modal.content.innerHTML = `
            <button class="modal-close-btn" onclick="closeModal('${modal.overlay.id}')">&times;</button>
            <h2 style="color: #f87171;">${title}</h2>
            <div>${content}</div>`;
        modal.overlay.classList.add('visible');
    }
    window.closeModal = (overlayId) => {
        const overlay = document.getElementById(overlayId);
        overlay.classList.remove('visible');
    };
    document.querySelectorAll('.modal-overlay').forEach(overlay => {
        overlay.addEventListener('click', (e) => {
            if (e.target === overlay) closeModal(overlay.id);
        });
    });

    // --- Cytoscape (Visualizer) Setup ---
    const cy = cytoscape({
        container: document.getElementById('cy'),
        style: [
            { selector: 'node', style: { 'background-color': '#1e293b', 'border-color': '#475569', 'border-width': 2, 'label': 'data(label)', 'color': '#e2e8f0', 'font-size': '12px', 'text-valign': 'center', 'text-halign': 'center', 'width': '140px', 'height': '50px', 'shape': 'round-rectangle', 'transition-property': 'background-color, border-color', 'transition-duration': '0.3s' } },
            { selector: 'edge', style: { 'width': 2, 'line-color': '#64748b', 'target-arrow-color': '#64748b', 'target-arrow-shape': 'triangle', 'curve-style': 'bezier' } },
            { selector: 'node:selected', style: { 'border-color': '#818cf8', 'background-color': '#312e81' } },
            { selector: '.status-succeeded', style: { 'border-color': '#22c55e' } },
            { selector: '.status-running', style: { 'border-color': '#f59e0b', 'line-style': 'dashed', 'border-dash-pattern': [6, 3] } },
            { selector: '.status-failed', style: { 'border-color': '#ef4444' } },
            { selector: '.status-pending', style: { 'border-color': '#64748b' } },
        ],
        layout: { name: 'dagre' }
    });

    // --- SVM Logic ---
    function stopSvmPolling() {
        if (svmPollingIntervalId) {
            clearTimeout(svmPollingIntervalId);
            svmPollingIntervalId = null;
        }
    }

    async function fetchSvmResults(workflowName, namespace) {
        dom.svm.statusText.textContent = 'Fetching final results...';
        const plotArtifactName = dom.svm.outputPlotName.value.trim();
        const metricsArtifactName = "metrics.json";
        try {
            const plotResponse = await fetch(`${API_BASE_URL}/api/workflows/${namespace}/${workflowName}/artifacts/${plotArtifactName}`);
            const metricsResponse = await fetch(`${API_BASE_URL}/api/workflows/${namespace}/${workflowName}/artifacts/${metricsArtifactName}`);
            if (!plotResponse.ok || !metricsResponse.ok) {
                throw new Error(`Failed to fetch artifacts. Plot: ${plotResponse.status}, Metrics: ${metricsResponse.status}`);
            }
            const plotBlob = await plotResponse.blob();
            dom.svm.resultPlot.src = URL.createObjectURL(plotBlob);
            const metricsJson = await metricsResponse.json();
            dom.svm.resultMetrics.textContent = JSON.stringify(metricsJson, null, 2);
            dom.svm.resultsArea.classList.remove('hidden');
            dom.svm.statusDisplay.classList.add('hidden');
            dom.svm.errorDisplay.classList.add('hidden');
        } catch(error) {
            dom.svm.errorText.textContent = `Could not fetch final results. Please check the visualizer for task status. Error: ${error.message}`;
            dom.svm.errorDisplay.classList.remove('hidden');
            dom.svm.statusDisplay.classList.add('hidden');
        } finally {
            dom.svm.runButton.disabled = false;
        }
    }

    function pollSvmStatus(workflowName, namespace) {
        stopSvmPolling();
        svmPollingIntervalId = setTimeout(async () => {
            const apiUrl = `${API_BASE_URL}/api/workflows/${workflowName}?namespace=${namespace}`;
            try {
                const response = await fetch(apiUrl);
                if (!response.ok) {
                    throw new Error(`Server responded with ${response.status}`);
                }
                const workflow = await response.json();
                const phase = workflow.status?.phase;
                dom.svm.statusText.textContent = `Status: ${phase || 'Unknown'}`;
                if (phase === 'Succeeded') {
                    stopSvmPolling();
                    fetchSvmResults(workflowName, namespace);
                } else if (phase === 'Failed' || phase === 'Error') {
                    stopSvmPolling();
                    dom.svm.errorText.textContent = workflow.status?.message || `Workflow ${phase}. Check logs for details.`;
                    dom.svm.errorDisplay.classList.remove('hidden');
                    dom.svm.statusDisplay.classList.add('hidden');
                    dom.svm.runButton.disabled = false;
                } else {
                    pollSvmStatus(workflowName, namespace);
                }
            } catch (error) {
                stopSvmPolling();
                dom.svm.errorText.textContent = `Error polling workflow status: ${error.message}.`;
                dom.svm.errorDisplay.classList.remove('hidden');
                dom.svm.statusDisplay.classList.add('hidden');
                dom.svm.runButton.disabled = false;
            }
        }, 5000);
    }

    async function handleRunExperiment() {
        stopSvmPolling();
        dom.svm.runButton.disabled = true;
        dom.svm.statusDisplay.classList.remove('hidden');
        dom.svm.statusText.textContent = 'Submitting workflow...';
        dom.svm.errorDisplay.classList.add('hidden');
        dom.svm.resultsArea.classList.add('hidden');
        const payload = {
            apiVersion: "qflow.io/v1alpha1",
            kind: "QuantumWorkflow",
            metadata: {
                name: `qsvm-${dom.svm.outputModelName.value.trim()}-${Date.now()}`,
                namespace: "default"
            },
            spec: {
                tasks: [{
                    name: "quantum-svm-pipeline",
                    spec: {
                        qsvm: {
                            dataset: {
                                generator: dom.svm.dsGenerator.value,
                                samples: parseInt(dom.svm.dsSamples.value, 10),
                                noise: parseFloat(dom.svm.dsNoise.value),
                                test_size: parseFloat(dom.svm.dsTestSize.value)
                            },
                            kernel: {
                                image: dom.svm.kernelImage.value
                            },
                            trainer: {
                                c: parseFloat(dom.svm.trainerC.value)
                            },
                            output: {
                                model_name: dom.svm.outputModelName.value,
                                plot_name: dom.svm.outputPlotName.value
                            }
                        }
                    }
                }]
            }
        };
        const namespace = payload.metadata.namespace;
        const apiUrl = `${API_BASE_URL}/api/workflows/${namespace}`;
        try {
            const response = await fetch(apiUrl, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(payload)
            });
            if (!response.ok) {
                const errorText = await response.text();
                throw new Error(`Server responded with ${response.status}. Message: ${errorText || 'No details provided.'}`);
            }
            const result = await response.json();
            dom.svm.statusText.textContent = 'Workflow submitted. Waiting for completion...';
            pollSvmStatus(result.metadata.name, result.metadata.namespace);
        } catch (error) {
            dom.svm.errorText.textContent = error.message;
            dom.svm.errorDisplay.classList.remove('hidden');
            dom.svm.statusDisplay.classList.add('hidden');
            dom.svm.runButton.disabled = false;
        }
    }
    dom.svm.runButton.addEventListener('click', handleRunExperiment);

    // --- ML SVM Logic ---
    function showMlError(msg) {
        dom.ml.errorDisplay.textContent = msg;
        dom.ml.errorDisplay.classList.remove('hidden');
    }

    async function handleMlRun() {
        const fileInput = document.getElementById('ml-data-file');
        const targetColumn = document.getElementById('ml-target-column').value.trim();
        const testSize = document.getElementById('ml-test-size').value;
        if (!targetColumn || !testSize) {
            showMlError('Please fill in all required fields.');
            return;
        }
        const formData = new FormData();
        if (fileInput.files.length > 0) {
            formData.append('data_file', fileInput.files[0]);
        }
        formData.append('target_column', targetColumn);
        formData.append('test_size', testSize);
        dom.ml.runButton.textContent = 'Running...';
        dom.ml.runButton.disabled = true;
        try {
            const response = await fetch(`${API_BASE_URL}/api/ml/svm`, {
                method: 'POST',
                body: formData
            });
            if (!response.ok) {
                const errorText = await response.text();
                throw new Error(`Server responded with ${response.status}. Message: ${errorText}`);
            }
            const result = await response.json();
            dom.ml.metrics.textContent = result.metrics;
            dom.ml.plot.src = `data:image/png;base64,${result.plot_base64}`;
            dom.ml.resultsArea.classList.remove('hidden');
            dom.ml.errorDisplay.classList.add('hidden');
        } catch (error) {
            showMlError(error.message);
        } finally {
            dom.ml.runButton.textContent = 'Run SVM';
            dom.ml.runButton.disabled = false;
        }
    }
    dom.ml.runButton.addEventListener('click', handleMlRun);

    // --- QCBM Submitter Logic ---
    function showQcbmNotification(message, type = 'success') {
        const notification = dom.qcbm.notification;
        notification.textContent = message;
        notification.className = 'p-4 mb-4 rounded-md';
        if (type === 'success') {
            notification.classList.add('bg-green-500/20', 'text-green-300');
        } else {
            notification.classList.add('bg-red-500/20', 'text-red-300');
        }
        notification.classList.remove('hidden');
    }

    async function handleQcbmSubmit() {
        const { qcbm } = dom;
        const workflowName = qcbm.workflowName.value.trim();
        const namespace = qcbm.namespace.value.trim();
        const taskName = qcbm.taskName.value.trim();
        const image = qcbm.image.value.trim();
        const ansatz = qcbm.ansatz.value.trim();
        if (!workflowName || !namespace || !taskName || !image || !ansatz) {
            showQcbmNotification('Please fill in all required fields.', 'error');
            return;
        }
        const trainingData = qcbm.trainingData.value.split('\n').filter(line => line.trim() !== '');
        const payload = {
            apiVersion: "qflow.io/v1alpha1",
            kind: "QuantumWorkflow",
            metadata: { name: workflowName, namespace: namespace },
            spec: {
                tasks: [{
                    name: taskName,
                    spec: {
                        qcbm: {
                            image: image,
                            ansatz: ansatz,
                            trainingData: trainingData,
                            optimizer: {
                                name: qcbm.optName.value.trim(),
                                epochs: parseInt(qcbm.optEpochs.value, 10),
                                learningRate: parseFloat(qcbm.optLr.value),
                                initialParams: qcbm.optParams.value.trim() || null
                            }
                        }
                    }
                }]
            }
        };
        qcbm.submitButton.textContent = 'Submitting...';
        qcbm.submitButton.disabled = true;
        const apiUrl = `${API_BASE_URL}/api/workflows/${namespace}`;
        try {
            const response = await fetch(apiUrl, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(payload)
            });
            if (!response.ok) {
                const errorText = await response.text();
                throw new Error(`Server responded with ${response.status}. Message: ${errorText}`);
            }
            const result = await response.json();
            showQcbmNotification(`Workflow "${result.metadata.name}" created successfully! Switching to visualizer...`, 'success');
            setTimeout(() => {
                dom.visualizer.workflowInput.value = workflowName;
                dom.visualizer.namespaceInput.value = namespace;
                showPage('visualizer');
                handleFetchWorkflow(false);
                qcbm.notification.classList.add('hidden');
            }, 2000);
        } catch (error) {
            showQcbmNotification(`Error: ${error.message}`, 'error');
        } finally {
            qcbm.submitButton.textContent = 'Submit Workflow';
            qcbm.submitButton.disabled = false;
        }
    }
    dom.qcbm.submitButton.addEventListener('click', handleQcbmSubmit);

    // --- Visualizer Event Handlers ---
    dom.visualizer.fetchButton.addEventListener('click', () => handleFetchWorkflow(false));
    dom.visualizer.workflowInput.addEventListener('keyup', (e) => { if (e.key === 'Enter') handleFetchWorkflow(false); });
    dom.visualizer.namespaceInput.addEventListener('keyup', (e) => { if (e.key === 'Enter') handleFetchWorkflow(false); });
    cy.on('tap', 'node', (evt) => displayTaskDetails(evt.target));
    cy.on('tap', (evt) => { if (evt.target === cy) resetDetailsPanel(); });
    dom.resultsModal.tabVis.addEventListener('click', (e) => { e.preventDefault(); switchResultTabs(true); });
    dom.resultsModal.tabLog.addEventListener('click', (e) => { e.preventDefault(); switchResultTabs(false); });

    // ...rest of the script logic from the HTML file...
});
