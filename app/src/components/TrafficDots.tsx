import type { TaskStatus } from "../types";

type Props = {
  status: TaskStatus;
};

export function TrafficDots({ status }: Props) {
  return (
    <div className={`traffic-dots traffic-dots--${status}`} aria-label={status}>
      <span className="traffic-dot traffic-dot--green" />
      <span className="traffic-dot traffic-dot--yellow" />
      <span className="traffic-dot traffic-dot--red" />
    </div>
  );
}
