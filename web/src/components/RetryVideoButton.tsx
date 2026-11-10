import { useState } from "react";
import { RotateCcw } from "lucide-react";
import { useRetryVideo } from "@/api/queries";
import { Button } from "@/components/ui/button";
import { errorMessage } from "@/lib/errorMessage";

interface RetryVideoButtonProps {
  videoId: string;
  /** Names the video in the failure alert. */
  title: string;
}

/**
 * The video detail pane's "Retry now" button: retries every retrying or
 * failed step of the video at once, then shows the video as refetched.
 */
export function RetryVideoButton({ videoId, title }: RetryVideoButtonProps) {
  const retryVideo = useRetryVideo();
  const [retrying, setRetrying] = useState(false);

  return (
    <Button
      variant="outline"
      size="sm"
      className="mb-2"
      disabled={retrying}
      onClick={async () => {
        setRetrying(true);
        try {
          await retryVideo(videoId);
        } catch (err) {
          window.alert(`Failed to retry "${title}": ${errorMessage(err)}`);
        } finally {
          setRetrying(false);
        }
      }}
    >
      <RotateCcw />
      Retry now
    </Button>
  );
}
