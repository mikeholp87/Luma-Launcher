using System.Globalization;

namespace Luma;

/// <summary>
/// + - * / ^, postfix %, parentheses, 0x/0b literals, pi, e and a few functions.
/// Parsed by hand; nothing is ever executed. Mirrors the Linux calculator.
/// </summary>
internal sealed class ArithmeticParser(string text)
{
    private int index;
    private int Length => text.Length;
    private void Whitespace() { while (index < text.Length && char.IsAscii(text[index]) && char.IsWhiteSpace(text[index])) index++; }
    private bool Take(char token) { Whitespace(); if (index >= text.Length || text[index] != token) return false; index++; return true; }
    // "200 + 10%" adds ten percent of 200, as on a phone calculator.
    private double Expression(int depth)
    {
        double result = Term(depth).Value;
        while (true)
        {
            double sign;
            if (Take('+')) sign = 1; else if (Take('-')) sign = -1; else return result;
            var (value, percent) = Term(depth);
            result += sign * (percent ? result * value : value);
        }
    }
    private (double Value, bool Percent) Term(int depth)
    {
        var (result, percent) = Factor(depth);
        while (true)
        {
            if (Take('*')) result *= Factor(depth).Value;
            else if (Take('/')) { double divisor = Factor(depth).Value; if (divisor == 0) throw new FormatException("Division by zero"); result /= divisor; }
            else return (result, percent);
            percent = false;
        }
    }
    private (double Value, bool Percent) Factor(int depth)
    {
        double value = Unary(depth);
        return Take('%') ? (value / 100, true) : (value, false);
    }
    // Signs bind looser than ^, so -2^2 is -4.
    private double Unary(int depth)
    {
        if (depth > 24) throw new FormatException("Too many parentheses");
        if (Take('-')) return -Unary(depth + 1);
        if (Take('+')) return Unary(depth + 1);
        return Power(depth);
    }
    // Right-associative: 2^3^2 is 2^9.
    private double Power(int depth)
    {
        double b = Atom(depth);
        return Take('^') ? Math.Pow(b, Unary(depth + 1)) : b;
    }
    private double Atom(int depth)
    {
        if (Take('(')) { double result = Expression(depth + 1); if (!Take(')')) throw new FormatException("Missing closing parenthesis"); return result; }
        Whitespace(); int start = index;
        if (index < text.Length && char.IsAsciiLetter(text[index]))
        {
            while (index < text.Length && char.IsAsciiLetter(text[index])) index++;
            string name = text[start..index];
            if (name == "pi") return Math.PI;
            if (name == "e") return Math.E;
            Func<double, double> function = name switch
            {
                "sqrt" => Math.Sqrt, "abs" => Math.Abs, "round" => x => Math.Round(x, MidpointRounding.AwayFromZero),
                "floor" => Math.Floor, "ceil" => Math.Ceiling, "ln" => Math.Log, "log" => Math.Log10,
                "sin" => Math.Sin, "cos" => Math.Cos, "tan" => Math.Tan,
                _ => throw new FormatException("Unknown name"),
            };
            if (!Take('(')) throw new FormatException("Functions need parentheses, such as sqrt(2)");
            double argument = Expression(depth + 1);
            if (!Take(')')) throw new FormatException("Missing closing parenthesis");
            return function(argument);
        }
        if (index + 1 < text.Length && text[index] == '0' && "xXbB".Contains(text[index + 1]))
        {
            int radix = char.ToLowerInvariant(text[index + 1]) == 'x' ? 16 : 2;
            index += 2; int digits = index;
            while (index < text.Length && char.IsAsciiHexDigit(text[index])) index++;
            try { return Convert.ToUInt64(text[digits..index], radix); }
            catch (Exception e) when (e is FormatException or ArgumentException or OverflowException) { throw new FormatException("Expected a number"); }
        }
        while (index < text.Length && (char.IsAsciiDigit(text[index]) || text[index] == '.')) index++;
        if (!double.TryParse(text[start..index], NumberStyles.AllowDecimalPoint, CultureInfo.InvariantCulture, out double number)) throw new FormatException("Expected a number");
        return number;
    }
    internal static double Calculate(string expression)
    {
        if (expression.Length > 256 || !expression.All(char.IsAscii)) throw new FormatException("Expression too long or invalid");
        var parser = new ArithmeticParser(expression.Trim());
        double result = parser.Expression(0); parser.Whitespace();
        if (parser.index != parser.Length) throw new FormatException("Invalid expression");
        if (!double.IsFinite(result)) throw new FormatException("Result is undefined");
        return result;
    }
}
