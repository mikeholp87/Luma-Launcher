using System.Globalization;

namespace Luma;

internal sealed class ArithmeticParser(string text)
{
    private int index;
    private void Whitespace() { while (index < text.Length && char.IsAscii(text[index]) && char.IsWhiteSpace(text[index])) index++; }
    private bool Take(char token) { Whitespace(); if (index >= text.Length || text[index] != token) return false; index++; return true; }
    private double Expression(int depth)
    {
        double result = Term(depth);
        while (true) { if (Take('+')) result += Term(depth); else if (Take('-')) result -= Term(depth); else return result; }
    }
    private double Term(int depth)
    {
        double result = Atom(depth);
        while (true)
        {
            if (Take('*')) result *= Atom(depth);
            else if (Take('/')) { double divisor = Atom(depth); if (divisor == 0) throw new FormatException("Division by zero"); result /= divisor; }
            else return result;
        }
    }
    private double Atom(int depth)
    {
        if (depth > 24) throw new FormatException("Expression is too deep");
        if (Take('-')) return -Atom(depth + 1);
        if (Take('+')) return Atom(depth + 1);
        if (Take('(')) { double result = Expression(depth + 1); if (!Take(')')) throw new FormatException("Missing closing parenthesis"); return result; }
        Whitespace(); int start = index;
        while (index < text.Length && (char.IsAsciiDigit(text[index]) || text[index] == '.')) index++;
        if (!double.TryParse(text[start..index], NumberStyles.AllowDecimalPoint, CultureInfo.InvariantCulture, out double number)) throw new FormatException("Expected a number");
        return number;
    }
    internal static double Calculate(string expression)
    {
        if (expression.Length > 256 || !expression.All(char.IsAscii)) throw new FormatException("Expression too long or invalid");
        var parser = new ArithmeticParser(expression);
        double result = parser.Expression(0); parser.Whitespace();
        if (parser.index != expression.Length || !double.IsFinite(result)) throw new FormatException("Invalid expression");
        return result;
    }
}
