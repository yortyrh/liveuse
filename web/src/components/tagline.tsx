"use client";

import { useLayoutEffect, useRef } from "react";

const lines = ["The agent gets a pointer.", "Only the token can use it."];
const rows = lines.map((line) => line.split(" "));

export function Tagline() {
  return (
    <div className="flex max-w-[680px] flex-col gap-2">
      {rows.map((words, lineIndex) => {
        const start = rows.slice(0, lineIndex).reduce((sum, line) => sum + line.length, 0);
        return (
          <p key={lines[lineIndex]} className="text-4xl font-semibold text-balance sm:text-5xl">
            {words.map((word, wordIndex) => (
              <Word key={`${start + wordIndex}`} word={word} index={start + wordIndex} />
            ))}
          </p>
        );
      })}
    </div>
  );
}

function Word({ word, index }: { word: string; index: number }) {
  const ref = useRef<HTMLSpanElement>(null);

  useLayoutEffect(() => {
    const node = ref.current;
    if (!node) return;
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;

    node.classList.add("tag-pending");
    let timeout = 0;
    const observer = new IntersectionObserver(
      ([entry]) => {
        if (!entry?.isIntersecting) return;
        observer.disconnect();
        timeout = window.setTimeout(() => node.classList.add("tag-on"), index * 50);
      },
      { rootMargin: "0px 0px -42% 0px", threshold: 0.5 },
    );
    observer.observe(node);
    return () => {
      observer.disconnect();
      window.clearTimeout(timeout);
    };
  }, [index]);

  return (
    <>
      <span ref={ref} className="inline-block">
        {word}
      </span>{" "}
    </>
  );
}
