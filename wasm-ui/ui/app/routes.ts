import { type RouteConfig, route } from "@react-router/dev/routes";

export default [
  route("", "routes/_layout.tsx", [
    route("", "routes/home.tsx"), // index route
    route("visualizer", "routes/visualizer.tsx"),
    route("svm", "routes/svm.tsx"),
    route("qcbm", "routes/qcbm.tsx"),
    route("qasm", "routes/qasm.tsx"),
    route("ml-svm", "routes/ml-svm.tsx"),
    route("simulator", "routes/simulator.tsx"),
  ]),
] as const;
